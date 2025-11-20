use anyhow::Result;
use clap::Parser;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_keypair::Keypair;
use solana_sdk::{clock::Clock, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{collections::HashSet, str::FromStr};
use std::{fs::File, io::Write};
use tokio::task::JoinSet;

mod adapter;
mod config;
mod constants;
mod svm;
mod swap;
mod utils;

use crate::{
    svm::{make_ata_account, token_balance},
    utils::{get_adapter, get_ata, get_token_mint},
};
use config::Config;

#[derive(Parser)]
#[command(name = "dex-tools")]
#[command(about = "Multi-protocol DEX tools for reading pool data and building swap instructions")]
struct Args {
    /// Protocol to use (tessera, orca, raydium, etc.)
    #[arg(short, long, default_value = "tessera")]
    protocol: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Read pool account data
    Read {
        /// Pool address to read
        #[arg(short, long)]
        pool_address: String,
    },
    /// Build swap instruction
    Swap {
        /// Input mint
        #[arg(short, long)]
        input_mint: String,
        /// Output mint
        #[arg(short, long)]
        output_mint: String,
        /// Pool address
        #[arg(short, long)]
        pool_address: Option<String>,

        /// Amount in (in token units)
        #[arg(short, long)]
        amount_in: u64,
        /// Minimum amount out (slippage protection)
        #[arg(short, long, default_value = "0")]
        min_amount_out: u64,
    },
    /// Simulate swap transaction
    Simulate {
        /// Input mint
        #[arg(short, long)]
        input_mint: String,
        /// Output mint
        #[arg(short, long)]
        output_mint: String,
        /// Pool address
        #[arg(short, long)]
        pool_address: Option<String>,

        /// Amount in (in token units)
        #[arg(short, long)]
        amount_in: u64,
        /// Minimum amount out (slippage protection)
        #[arg(short, long, default_value = "0")]
        min_amount_out: u64,
    },
    /// List pool accounts for a protocol
    List {
        /// Protocol to list pools for
        #[arg(short, long, default_value = "tessera")]
        protocol: String,
    },
    /// Dump all pool accounts to external file
    Dump {
        /// Pool address
        #[arg(short, long)]
        pool_address: String,

        /// Output file path
        #[arg(short, long, default_value = "pool_data.json")]
        output: String,
    },
    /// Simulate curve swap
    CurveSimulate {
        /// Pool address
        #[arg(short, long)]
        pool_address: String,

        /// Swap direction (aTob or bToa)
        #[arg(short, long)]
        a_to_b: u8,

        #[arg(short, long, default_value = "pool_data.json")]
        file: String,
        /// CSV output file path
        #[arg(short = 'o', long, default_value = "result.csv")]
        csv_out: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::from_env()?;

    match args.command {
        Commands::List { protocol } => handle_list_command(&protocol, &config).await,
        Commands::Read { pool_address } => {
            handle_read_command(&args.protocol, pool_address, &config).await
        }
        Commands::Simulate {
            input_mint,
            output_mint,
            pool_address,
            amount_in,
            min_amount_out,
        } => {
            handle_swap_command(
                &args.protocol,
                input_mint,
                output_mint,
                pool_address,
                amount_in,
                min_amount_out,
                &config,
                true,
            )
            .await
        }
        Commands::Swap {
            input_mint,
            output_mint,
            pool_address,
            amount_in,
            min_amount_out,
        } => {
            handle_swap_command(
                &args.protocol,
                input_mint,
                output_mint,
                pool_address,
                amount_in,
                min_amount_out,
                &config,
                false,
            )
            .await
        }
        Commands::Dump {
            pool_address,
            output,
        } => handle_dump_command(&args.protocol, pool_address, output, &config).await,
        Commands::CurveSimulate {
            pool_address,
            a_to_b,
            file,
            csv_out,
        } => {
            handle_curve_simulate_command(
                &args.protocol,
                pool_address,
                a_to_b == 1,
                &config,
                file,
                csv_out,
            )
            .await
        }
    }
}

async fn handle_read_command(protocol: &str, pool_address: String, config: &Config) -> Result<()> {
    let pool_address = Pubkey::from_str(&pool_address)?;

    let adapter = get_adapter(protocol, config)?;
    match adapter.fetch_pool_data(&pool_address, config) {
        Ok(data) => {
            data.display();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    Ok(())
}

async fn handle_list_command(protocol: &str, config: &Config) -> Result<()> {
    let adapter = get_adapter(protocol, config)?;
    let pools = adapter.get_pools(config)?;

    println!("\nFound {} pool accounts:", pools.len());
    println!("{}", "=".repeat(80));

    for (i, pubkey) in pools.iter().enumerate() {
        println!("{}. {}", i + 1, pubkey);
    }

    if pools.is_empty() {
        anyhow::bail!("No pool accounts found for {} protocol.", protocol);
    }

    Ok(())
}

async fn handle_dump_command(
    protocol: &str,
    pool_address: String,
    output_file: String,
    config: &Config,
) -> Result<()> {
    let adapter = get_adapter(protocol, config)?;
    let pool_address = Pubkey::from_str(&pool_address)?;
    let program_id = adapter.get_program_id();
    let pool_data = adapter.fetch_pool_data(&pool_address, config)?;
    pool_data.display();

    let swap_ixs = adapter.build_swap_instructions(
        pool_data.as_ref(),
        &Pubkey::default(),
        true,
        1_000_000,
        0,
    )?;

    let mut addresses = HashSet::new();
    for ix in swap_ixs {
        for account in ix.accounts {
            if account.pubkey.eq(&program_id) {
                continue;
            }

            addresses.insert(account.pubkey);
        }
    }

    addresses.insert(pool_data.get_mint_a());
    addresses.insert(pool_data.get_mint_b());

    // Get all PDAs
    let rpc_client = RpcClient::new(config.get_rpc_url());
    let pda_accounts = rpc_client.get_program_accounts(&program_id)?;
    for pda in pda_accounts {
        addresses.insert(pda.0);
    }

    svm::dump_pool_accounts(
        addresses.into_iter().collect::<Vec<_>>(),
        config.get_rpc_url(),
        &output_file,
    )
}

async fn handle_swap_command(
    protocol: &str,
    input_mint: String,
    output_mint: String,
    pool_address: Option<String>,
    amount_in: u64,
    min_amount_out: u64,
    config: &Config,
    is_simulate: bool,
) -> Result<()> {
    let input_mint = get_token_mint(&input_mint)?;
    let output_mint = get_token_mint(&output_mint)?;

    let adapter = get_adapter(protocol, config)?;

    let pool_data = match pool_address {
        Some(pool_address) => adapter.fetch_pool_data(&Pubkey::from_str(&pool_address)?, config)?,
        None => adapter.fetch_pair_data(&input_mint, &output_mint, config)?,
    };
    pool_data.display();

    let a_to_b = pool_data.get_mint_a().eq(&input_mint);

    // Build the transaction using the swap module
    let transaction = swap::build_swap_transaction(
        adapter,
        pool_data,
        a_to_b,
        amount_in,
        min_amount_out,
        config,
    )?;

    let client = RpcClient::new(config.get_rpc_url());

    if is_simulate {
        println!("\nSimulating Transaction...");
        // Simulate the transaction
        let simulation_result = client.simulate_transaction_with_config(
            &transaction,
            RpcSimulateTransactionConfig {
                replace_recent_blockhash: true,
                sig_verify: false,
                ..RpcSimulateTransactionConfig::default()
            },
        )?;
        match simulation_result.value.err {
            None => {
                println!("✅ Transaction simulation successful!");
                println!(
                    "Compute Units Consumed: {}",
                    simulation_result.value.units_consumed.unwrap_or(0)
                );
                println!("Logs:");
                for log in simulation_result.value.logs.unwrap_or_default() {
                    println!("  {}", log);
                }
            }
            Some(err) => {
                println!("❌ Transaction simulation failed: {:?}", err);
                println!("Logs:");
                for log in simulation_result.value.logs.unwrap_or_default() {
                    println!("  {}", log);
                }
            }
        }
    } else {
        println!("\nSubmitting Transaction...");
        let signature = client.send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;
        println!("Transaction submitted: {}", signature);
    }

    Ok(())
}

async fn handle_curve_simulate_command(
    protocol: &str,
    pool_address: String,
    a_to_b: bool,
    config: &Config,
    file: String,
    csv_out: String,
) -> Result<()> {
    println!("CurveSimulate command called:");
    println!("  Protocol: {}", protocol);
    println!("  Pool Address: {}", pool_address);
    println!("  Direction: {}", a_to_b);

    let pool_address = Pubkey::from_str(&pool_address)?;
    let mut csv = File::create(&csv_out)?;
    writeln!(csv, "direction,in_amount,out_amount,price,oracle_price")?;
    let adapter = get_adapter(protocol, config)?;

    let svm = svm::init_svm(&file)?;
    let pool_data = adapter.load_pool_data(&pool_address, &svm)?;

    let oracle_price = pool_data.as_ref().get_oracle_price();
    let oracle_price = if a_to_b {
        oracle_price
    } else {
        1.0 / oracle_price
    };
    println!("Original price: {:?}", oracle_price);

    let (input_token, output_token) = if a_to_b {
        (pool_data.get_mint_a(), pool_data.get_mint_b())
    } else {
        (pool_data.get_mint_b(), pool_data.get_mint_a())
    };

    let (input_decimals, output_decimals) = if a_to_b {
        (pool_data.get_decimals_a(), pool_data.get_decimals_b())
    } else {
        (pool_data.get_decimals_b(), pool_data.get_decimals_a())
    };

    let (input_vault, output_vault) = if a_to_b {
        (pool_data.get_vault_a(), pool_data.get_vault_b())
    } else {
        (pool_data.get_vault_b(), pool_data.get_vault_a())
    };

    let input_vault_balance =
        token_balance(&svm, &input_vault) as f64 / 10f64.powi(input_decimals as i32);
    let output_vault_balance =
        token_balance(&svm, &output_vault) as f64 / 10f64.powi(output_decimals as i32);
    println!("Input vault balance: {}", input_vault_balance);
    println!("Output vault balance: {}", output_vault_balance);

    // Initialize SVM with the dumped accounts
    let user_keypair = Keypair::new();
    let user = user_keypair.pubkey();

    let mut in_amounts = vec![
        1_000,
        // 10_000,
        // 100_000,
        // 1_000_000,
        // 10_000_000,
        // 100_000_000,
        // 1_000_000_000,
        // 10_000_000_000,
        // 100_000_000_000,
        // 1_000_000_000_000,
        // 10_000_000_000_000,
    ];

    let min_amount_out = 1;

    // let blockhash = svm.get_sysvar::<solana_sdk::sysvar::instructions::Instructions>();
    // println!("Blockhash: {}", blockhash);

    // Run SVM simulations in parallel; each task owns its own SVM and input
    let mut tasks = JoinSet::new();
    for &in_amount in &in_amounts {
        let file = file.clone();
        let protocol = protocol.to_string();
        let config = config.clone();
        let pool_address = pool_address;
        let input_token = input_token;
        let output_token = output_token;
        let input_decimals = input_decimals;
        let output_decimals = output_decimals;
        let a_to_b = a_to_b;
        let oracle_price = oracle_price;

        tasks.spawn_blocking(move || -> Result<String> {
            // New user per task
            let user_keypair = Keypair::new();
            let user = user_keypair.pubkey();
            let mut svm = svm::init_svm(&file)?;
            svm.airdrop(&user, LAMPORTS_PER_SOL)
                .map_err(|e| anyhow::anyhow!("Failed to airdrop: {:?}", e))?;

            // Build instructions inside the task
            let adapter = get_adapter(&protocol, &config)?;
            let pool_data = adapter.as_ref().load_pool_data(&pool_address, &svm)?;

            let (in_token_program, out_token_program) = if a_to_b {
                (
                    pool_data.get_token_program_a(),
                    pool_data.get_token_program_b(),
                )
            } else {
                (
                    pool_data.get_token_program_b(),
                    pool_data.get_token_program_a(),
                )
            };

            let user_in_ata = get_ata(&user, &input_token, &in_token_program);
            make_ata_account(
                &mut svm,
                &input_token,
                &user_keypair,
                &in_token_program,
                in_amount,
            );

            let user_out_ata = get_ata(&user, &output_token, &out_token_program);
            make_ata_account(
                &mut svm,
                &output_token,
                &user_keypair,
                &out_token_program,
                0,
            );

            let ixs = swap::build_simulate_ixs(
                &adapter,
                pool_data,
                &user,
                a_to_b,
                in_amount,
                min_amount_out,
            )?;

            for ix in ixs.iter() {
                for account in ix.accounts.iter() {
                    println!("{:?}", account.pubkey);
                    match svm.get_account(&account.pubkey) {
                        Some(acc) => {
                            println!("      Owner: {}", acc.owner.to_string());
                        }
                        None => {
                            println!("      Account not found: {:?}", account.pubkey);
                        }
                    }
                }
            }

            let tx = Transaction::new_with_payer(&ixs, Some(&user));
            let signed_tx = Transaction::new(&[&user_keypair], tx.message, svm.latest_blockhash());
            svm.simulate_transaction(signed_tx)
                .map_err(|e| anyhow::anyhow!("send failed: {:?}", e))?;

            let out_balance = svm::token_balance(&svm, &user_out_ata);
            let in_amount_f64 = in_amount as f64 / 10f64.powi(input_decimals as i32);
            let out_amount_f64 = out_balance as f64 / 10f64.powi(output_decimals as i32);
            let price = out_amount_f64 / in_amount_f64;

            let direction = if a_to_b { "AtoB" } else { "BtoA" };
            Ok(format!(
                "{},{:.10},{:.10},{:.10},{}",
                direction, in_amount_f64, out_amount_f64, price, oracle_price
            ))
        });
    }

    // Collect results and write to CSV
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(line)) => {
                writeln!(csv, "{}", line)?;
            }
            Ok(Err(e)) => {
                println!("Task failed: {}", e);
            }
            Err(e) => {
                println!("Join error: {}", e);
            }
        }
    }

    Ok(())
}
