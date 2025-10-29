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

mod adapter;
mod config;
mod constants;
mod utils;

use crate::{
    constants::WSOL_MINT,
    utils::{
        build_unwrap_sol_instruction, build_wrap_sol_instruction, get_ata, get_pubkey_from_str,
    },
};
use adapter::{
    alphaq::AlphaqAdapter, goonfi::GoonfiAdapter, obric::ObricAdapter, saros_amm::SarosAdapter,
    tessera::TesseraAdapter, DexAdapter,
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

fn get_adapter(protocol: &str, config: &Config) -> Result<Box<dyn DexAdapter>> {
    let client = RpcClient::new(config.get_rpc_url());

    let adapter: Box<dyn DexAdapter> = match protocol {
        "alphaq" => Box::new(AlphaqAdapter::new(client)),
        "tessera" => Box::new(TesseraAdapter::new(client)),
        "goonfi" => Box::new(GoonfiAdapter::new(client)),
        "obric" => Box::new(ObricAdapter::new(client)),
        "saros" => Box::new(SarosAdapter::new(client)),
        _ => {
            anyhow::bail!("Unsupported protocol: {}", protocol);
        }
    };

    Ok(adapter)
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

    let adapter = get_adapter(protocol, config)?;

    // First, read pool data to get the mints and current state
    let pool_data = adapter.fetch_pool_data(&pool_address, config)?;
    pool_data.display();

    // Build swap instructions
    let swap_ixs = adapter.build_swap_instructions(
        pool_data.as_ref(),
        &user,
        &input_token,
        amount_in,
        min_amount_out,
    )?;

    // Determine output token
    let output_token = if input_token.eq(&pool_data.get_mint_a()) {
        pool_data.get_mint_b()
    } else {
        pool_data.get_mint_a()
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
