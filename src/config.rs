use crate::constants::{
    DEFAULT_RPC_URL, GOONFI_PROGRAM_ID, OBRIC_PROGRAM_ID, TESSERA_AUTHORITY, TESSERA_PROGRAM_ID,
};
use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{env, str::FromStr};

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub tessera_program_id: String,
    pub tessera_authority: String,
    pub goonfi_program_id: String,
    pub obric_program_id: String,
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
        let payer_pk = env::var("WALLET_PRIVATE_KEY").unwrap_or_else(|_| "".to_string());

        Ok(Self {
            rpc_url,
            tessera_program_id,
            tessera_authority,
            goonfi_program_id,
            obric_program_id,
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

    pub fn get_tessera_program_id(&self) -> Pubkey {
        Pubkey::from_str(&self.tessera_program_id).unwrap()
    }

    pub fn get_tessera_authority(&self) -> Pubkey {
        Pubkey::from_str(&self.tessera_authority).unwrap()
    }

    pub fn get_goonfi_program_id(&self) -> Pubkey {
        Pubkey::from_str(&self.goonfi_program_id).unwrap()
    }

    pub fn get_obric_program_id(&self) -> Pubkey {
        Pubkey::from_str(&self.obric_program_id).unwrap()
    }
}
