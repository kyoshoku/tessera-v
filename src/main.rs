use anyhow::Result;
use clap::Parser;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_sdk::{
    message::{Message, VersionedMessage},
    pubkey::Pubkey,
    signature::Signer,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use std::str::FromStr;

mod config;
mod constants;
mod fetch;
mod swap;
mod utils;

use config::Config;
use fetch::tessera::{TesseraPool, TesseraPoolFetcher};
use fetch::PoolFetcher;
use swap::goonfi::GoonfiSwapBuilder;
use swap::tessera::TesseraSwapBuilder;
use swap::SwapBuilder;

use crate::{
    constants::WSOL_MINT,
    fetch::{
        alphaq::{AlphaqPool, AlphaqPoolFetcher},
        goonfi::{GoonfiPool, GoonfiPoolFetcher},
        obric::{ObricPool, ObricPoolFetcher},
        saros_amm::{SarosPool, SarosPoolFetcher},
    },
    swap::{alphaq::AlphaqSwapBuilder, obric::ObricSwapBuilder, saros_amm::SarosSwapBuilder},
    utils::{
        build_unwrap_sol_instruction, build_wrap_sol_instruction, get_ata, get_pubkey_from_str,
    },
};

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
        /// Pool address
        #[arg(short, long)]
        pool_address: String,

        /// Input token mint (base or quote)
        #[arg(short, long)]
        input_token: String,

        /// Amount in (in token units)
        #[arg(short, long)]
        amount_in: u64,

        /// Minimum amount out (slippage protection)
        #[arg(short, long, default_value = "0")]
        min_amount_out: u64,
    },
    /// Simulate swap transaction
    Simulate {
        /// Pool address
        #[arg(short, long)]
        pool_address: String,

        /// Input token mint (base or quote)
        #[arg(short, long)]
        input_token: String,

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::from_env()?;

    match args.command {
        Commands::Read { pool_address } => {
            handle_read_command(&args.protocol, pool_address, &config).await
        }
        Commands::Simulate {
            pool_address,
            input_token,
            amount_in,
            min_amount_out,
        } => {
            handle_swap_command(
                &args.protocol,
                pool_address,
                input_token,
                amount_in,
                min_amount_out,
                &config,
                true,
            )
            .await
        }
        Commands::List { protocol } => handle_list_command(&protocol, &config).await,
        Commands::Swap {
            pool_address,
            input_token,
            amount_in,
            min_amount_out,
        } => {
            handle_swap_command(
                &args.protocol,
                pool_address,
                input_token,
                amount_in,
                min_amount_out,
                &config,
                false,
            )
            .await
        }
    }
}

async fn handle_read_command(protocol: &str, pool_address: String, config: &Config) -> Result<()> {
    let pool_address = Pubkey::from_str(&pool_address)?;
    let client = RpcClient::new(config.get_rpc_url());

    match protocol {
        "alphaq" => {
            let fetcher = AlphaqPoolFetcher::new(client);
            match fetcher.fetch_pool_data(&pool_address, config).await {
                Ok(data) => {
                    data.display();
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        "tessera" => {
            let fetcher = TesseraPoolFetcher::new(client);
            match fetcher.fetch_pool_data(&pool_address, config).await {
                Ok(data) => {
                    data.display();
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        "goonfi" => {
            let fetcher = GoonfiPoolFetcher::new(client);
            match fetcher.fetch_pool_data(&pool_address, config).await {
                Ok(data) => {
                    data.display();
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        "obric" => {
            let fetcher = ObricPoolFetcher::new(client);
            match fetcher.fetch_pool_data(&pool_address, config).await {
                Ok(data) => {
                    data.display();
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        "saros" => {
            let fetcher = SarosPoolFetcher::new(client);
            match fetcher.fetch_pool_data(&pool_address, config).await {
                Ok(data) => {
                    data.display();
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        _ => {
            println!("Unsupported protocol: {}", protocol);
        }
    }

    Ok(())
}

async fn handle_swap_command(
    protocol: &str,
    pool_address: String,
    input_token: String,
    amount_in: u64,
    min_amount_out: u64,
    config: &Config,
    is_simulate: bool,
) -> Result<()> {
    let pool_address = get_pubkey_from_str(&pool_address)?;
    let input_token = get_pubkey_from_str(&input_token)?;
    let client = RpcClient::new(config.get_rpc_url());

    // Load payer from config
    let payer = config
        .get_payer()
        .map_err(|e| anyhow::anyhow!("Failed to get payer: {}", e))?;
    let user = payer.pubkey();
    println!("User: {}", user);

    let mut pre_ixs = vec![];

    let wsol_ata = get_associated_token_address_with_program_id(&user, &WSOL_MINT, &spl_token::ID);
    let wrap_sol = input_token.eq(&WSOL_MINT) || input_token.eq(&WSOL_MINT);
    if wrap_sol {
        let wrap_sol_amount = if input_token.eq(&WSOL_MINT) {
            amount_in
        } else {
            0
        };
        pre_ixs.extend(build_wrap_sol_instruction(
            &user,
            &wsol_ata,
            wrap_sol_amount,
        ));
    }

    let (output_token, swap_ixs) = match protocol {
        "tessera" => {
            // First, read pool data to get the mints and current state
            let fetcher = TesseraPoolFetcher::new(client);
            let pool_data = fetcher.fetch_pool_data(&pool_address, config).await?;
            pool_data.display();

            // Build swap instruction
            let pool_data = pool_data
                .as_any()
                .downcast_ref::<TesseraPool>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

            let builder = TesseraSwapBuilder::new(pool_data.clone(), user);
            let swap_ixs = builder.build_swap(&input_token, amount_in, min_amount_out)?;

            let output_token = if input_token.eq(&pool_data.mint_a) {
                pool_data.mint_b
            } else {
                pool_data.mint_a
            };
            (output_token, swap_ixs)
        }
        "goonfi" => {
            // First, read pool data to get the mints and current state
            let fetcher = GoonfiPoolFetcher::new(client);
            let pool_data = fetcher.fetch_pool_data(&pool_address, config).await?;
            pool_data.display();

            // Build swap instruction
            let pool_data = pool_data
                .as_any()
                .downcast_ref::<GoonfiPool>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

            let builder = GoonfiSwapBuilder::new(pool_data.clone(), user);
            let swap_ixs = builder.build_swap(&input_token, amount_in, min_amount_out)?;

            let output_token = if input_token.eq(&pool_data.mint_a) {
                pool_data.mint_b
            } else {
                pool_data.mint_a
            };
            (output_token, swap_ixs)
        }
        "obric" => {
            let fetcher = ObricPoolFetcher::new(client);
            let pool_data = fetcher.fetch_pool_data(&pool_address, config).await?;
            pool_data.display();

            // Build swap instruction
            let pool_data = pool_data
                .as_any()
                .downcast_ref::<ObricPool>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

            let builder = ObricSwapBuilder::new(pool_data.clone(), user);
            let swap_ixs = builder.build_swap(&input_token, amount_in, min_amount_out)?;

            let output_token = if input_token.eq(&pool_data.mint_a) {
                pool_data.mint_b
            } else {
                pool_data.mint_a
            };
            (output_token, swap_ixs)
        }
        "saros" => {
            let fetcher = SarosPoolFetcher::new(client);
            let pool_data = fetcher.fetch_pool_data(&pool_address, config).await?;
            pool_data.display();

            // Build swap instruction
            let pool_data = pool_data
                .as_any()
                .downcast_ref::<SarosPool>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

            let builder = SarosSwapBuilder::new(pool_data.clone(), user, config.clone());
            let swap_ixs = builder.build_swap(&input_token, amount_in, min_amount_out)?;

            let output_token = if input_token.eq(&pool_data.mint_a) {
                pool_data.mint_b
            } else {
                pool_data.mint_a
            };
            (output_token, swap_ixs)
        }
        "alphaq" => {
            let fetcher = AlphaqPoolFetcher::new(client);
            let pool_data = fetcher.fetch_pool_data(&pool_address, config).await?;
            pool_data.display();

            // Build swap instruction
            let pool_data = pool_data
                .as_any()
                .downcast_ref::<AlphaqPool>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

            let builder = AlphaqSwapBuilder::new(pool_data.clone(), user);
            let swap_ixs = builder.build_swap(&input_token, amount_in, min_amount_out)?;

            let output_token = if input_token.eq(&pool_data.mint_a) {
                pool_data.mint_b
            } else {
                pool_data.mint_a
            };
            (output_token, swap_ixs)
        }
        _ => {
            anyhow::bail!("Unsupported protocol: {}", protocol);
        }
    };

    let output_ata = get_ata(&user, &output_token, &spl_token::ID);
    if !output_ata.eq(&wsol_ata) {
        let client = RpcClient::new(config.get_rpc_url());
        let output_ata_exists = client.get_account(&output_ata).is_ok();
        if !output_ata_exists {
            pre_ixs.push(create_associated_token_account_idempotent(
                &user,
                &user,
                &output_token,
                &spl_token::ID,
            ));
        }
    }

    let post_ixs = if wrap_sol {
        build_unwrap_sol_instruction(&user, &wsol_ata)
    } else {
        vec![]
    };

    let ixs = [pre_ixs, swap_ixs, post_ixs].concat();

    let client = RpcClient::new(config.get_rpc_url());
    let recent_blockhash = client.get_latest_blockhash()?;

    // Build the transaction
    let message = Message::new_with_blockhash(&ixs, Some(&user), &recent_blockhash);

    let versioned_message = VersionedMessage::Legacy(message);
    let transaction = VersionedTransaction::try_new(versioned_message, &[&payer])?;

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

async fn handle_list_command(protocol: &str, config: &Config) -> Result<()> {
    let client = RpcClient::new(config.get_rpc_url());

    let pools = match protocol {
        "tessera" => {
            let fetcher = TesseraPoolFetcher::new(client);
            fetcher.get_pools(config).await?
        }
        "alphaq" => {
            let fetcher = AlphaqPoolFetcher::new(client);
            fetcher.get_pools(config).await?
        }
        "goonfi" => {
            let fetcher = GoonfiPoolFetcher::new(client);
            fetcher.get_pools(config).await?
        }
        "obric" => {
            let fetcher = ObricPoolFetcher::new(client);
            fetcher.get_pools(config).await?
        }
        "saros" => {
            let fetcher = SarosPoolFetcher::new(client);
            fetcher.get_pools(config).await?
        }
        _ => {
            anyhow::bail!("Unsupported protocol: {}", protocol);
        }
    };

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
