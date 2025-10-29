use crate::{
    config::Config,
    fetch::{PoolData, PoolFetcher},
    utils::get_pubkey_from_str,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;


#[derive(Debug, Clone)]
pub struct TesseraPool {
    pub pk: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub oracle_price: f64, // in USD
}

impl PoolData for TesseraPool {
    fn display(&self) {
        println!("Mint A: {}", self.mint_a);
        println!("Mint B: {}", self.mint_b);
        println!("Oracle Price: ${:.4}", self.oracle_price);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct TesseraPoolFetcher {
    client: RpcClient,
}

impl TesseraPoolFetcher {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

fn get_vault_address(mint: String) -> Pubkey {
    match mint.as_str() {
        "So11111111111111111111111111111111111111112" => {
            get_pubkey_from_str("5pVN5XZB8cYBjNLFrsBCPWkCQBan5K5Mq2dWGzwPgGJV").unwrap()
        }
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => {
            get_pubkey_from_str("9t4P5wMwfFkyn92Z7hf463qYKEZf8ERVZsGBEPNp8uJx").unwrap()
        }
        "METvsvVRapdj9cFLzq4Tr43xK4tAjQfwX76z3n6mWQL" => {
            get_pubkey_from_str("98zmEfUKzsyhWgd6udFrRhfEUbpaLbiS77y2ZCSTWgZk").unwrap()
        }
        _ => panic!("Unsupported vault for {}", mint),
    }
}

impl PoolFetcher for TesseraPoolFetcher {
    async fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let account = self.client.get_account(pool_address)?;
        let data = &account.data;

        // Parse according to Tessera structure
        // Discriminator @ bytes 0-7
        let _discriminator = data[0..8].to_vec();

        // Mint A @ byte 24 (32 bytes)
        let mint_a_bytes = &data[24..56];
        let mint_a = Pubkey::new_from_array(mint_a_bytes.try_into().unwrap());

        // Mint B @ byte 56 (32 bytes)
        let mint_b_bytes = &data[56..88];
        let mint_b = Pubkey::new_from_array(mint_b_bytes.try_into().unwrap());

        // Oracle Price @ byte 128 (8 bytes, u64 in pico-USDC)
        let oracle_price_pico = u64::from_le_bytes(data[128..136].try_into().unwrap());
        let oracle_price = oracle_price_pico as f64 / 1e12;

        // Get the mintA info
        let mint_a_account = self.client.get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = self.client.get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;

        // Hardcoded vault addresses
        let vault_a = get_vault_address(mint_a.to_string());
        let vault_b = get_vault_address(mint_b.to_string());

        Ok(Box::new(TesseraPool {
            pk: *pool_address,
            mint_a,
            mint_b,
            token_program_a,
            token_program_b,
            oracle_price,
            vault_a,
            vault_b,
        }))
    }

    async fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        use crate::constants::TESSERA_PROGRAM_ID;
        
        // Get all accounts owned by the Tessera program
        let accounts = self.client.get_program_accounts(&TESSERA_PROGRAM_ID)?;
        
        // Extract just the pubkeys
        let pubkeys: Vec<Pubkey> = accounts.into_iter().map(|(pubkey, _)| pubkey).collect();
        
        Ok(pubkeys)
    }
}
