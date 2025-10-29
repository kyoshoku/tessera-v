use crate::constants::{
    DEFAULT_RPC_URL, GOONFI_PROGRAM_ID, OBRIC_PROGRAM_ID, SAROS_PROGRAM_ID, TESSERA_AUTHORITY,
    TESSERA_PROGRAM_ID,
};
use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{env, str::FromStr};

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub tessera_program_id: Pubkey,
    pub tessera_authority: Pubkey,
    pub goonfi_program_id: Pubkey,
    pub obric_program_id: Pubkey,
    pub saros_program_id: Pubkey,
    pub payer_pk: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if it exists

        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        let tessera_program_id =
            env::var("TESSERA_PROGRAM_ID").unwrap_or_else(|_| TESSERA_PROGRAM_ID.to_string());
        let tessera_authority =
            env::var("TESSERA_AUTHORITY").unwrap_or_else(|_| TESSERA_AUTHORITY.to_string());
        let goonfi_program_id =
            env::var("GOONFI_PROGRAM_ID").unwrap_or_else(|_| GOONFI_PROGRAM_ID.to_string());
        let obric_program_id =
            env::var("OBRIC_PROGRAM_ID").unwrap_or_else(|_| OBRIC_PROGRAM_ID.to_string());
        let saros_program_id =
            env::var("SAROS_PROGRAM_ID").unwrap_or_else(|_| SAROS_PROGRAM_ID.to_string());
        let payer_pk = env::var("WALLET_PRIVATE_KEY").unwrap_or_else(|_| "".to_string());

        Ok(Self {
            rpc_url,
            tessera_program_id: Pubkey::from_str(&tessera_program_id).unwrap(),
            tessera_authority: Pubkey::from_str(&tessera_authority).unwrap(),
            goonfi_program_id: Pubkey::from_str(&goonfi_program_id).unwrap(),
            obric_program_id: Pubkey::from_str(&obric_program_id).unwrap(),
            saros_program_id: Pubkey::from_str(&saros_program_id).unwrap(),
            payer_pk,
        })
    }

    pub fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
    pub fn get_payer(&self) -> Result<Keypair> {
        let payer = Keypair::from_bytes(&bs58::decode(&self.payer_pk).into_vec()?)?;
        Ok(payer)
    }
}
