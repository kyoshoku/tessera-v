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
    fetch::{
        goonfi::{GoonfiPool, GoonfiPoolFetcher},
        obric::{ObricPool, ObricPoolFetcher},
    },
    swap::obric::ObricSwapBuilder,
    utils::get_pubkey_from_str,
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

    let swap_ixs = match protocol {
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

            let builder = TesseraSwapBuilder::new(pool_data.clone(), user, config.clone());
            builder.build_swap(&input_token, amount_in, min_amount_out, true)?
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

            let builder = GoonfiSwapBuilder::new(pool_data.clone(), user, config.clone());
            builder.build_swap(&input_token, amount_in, min_amount_out, true)?
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

            let builder = ObricSwapBuilder::new(pool_data.clone(), user, config.clone());
            builder.build_swap(&input_token, amount_in, min_amount_out, true)?
        }
        _ => {
            anyhow::bail!("Unsupported protocol: {}", protocol);
        }
    };

    let client = RpcClient::new(config.get_rpc_url());
    let recent_blockhash = client.get_latest_blockhash()?;

    // Build the transaction
    let message = Message::new_with_blockhash(&swap_ixs, Some(&user), &recent_blockhash);

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
