use crate::constants::DEFAULT_RPC_URL;
use anyhow::Result;
use solana_sdk::signature::Keypair;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub payer_pk: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if it exists

        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        let payer_pk = env::var("WALLET_PRIVATE_KEY").unwrap_or_else(|_| "".to_string());

        Ok(Self { rpc_url, payer_pk })
    }

    pub fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
    pub fn get_payer(&self) -> Result<Keypair> {
        let payer = Keypair::from_bytes(&bs58::decode(&self.payer_pk).into_vec()?)?;
        Ok(payer)
    }
}
