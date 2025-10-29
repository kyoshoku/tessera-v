use crate::{
    config::Config,
    fetch::{PoolData, PoolFetcher},
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct GoonfiPool {
    pub pk: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub oracle_price: f64, // in USD
}

impl PoolData for GoonfiPool {
    fn display(&self) {
        println!("Mint A: {}", self.mint_a);
        println!("Mint B: {}", self.mint_b);
        println!("Vault A: {}", self.vault_a);
        println!("Vault B: {}", self.vault_b);
        println!("Oracle Price: ${:.4}", self.oracle_price);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct GoonfiPoolFetcher {
    client: RpcClient,
}

impl GoonfiPoolFetcher {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl PoolFetcher for GoonfiPoolFetcher {
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

        // Mint A @ byte 256 (32 bytes)
        let mint_a_bytes = &data[256..288];
        let mint_a = Pubkey::new_from_array(mint_a_bytes.try_into().unwrap());

        // Mint B @ byte 288 (32 bytes)
        let mint_b_bytes = &data[288..320];
        let mint_b = Pubkey::new_from_array(mint_b_bytes.try_into().unwrap());

        // Vault A @ byte 320 (32 bytes)
        let vault_a_bytes = &data[320..352];
        let vault_a = Pubkey::new_from_array(vault_a_bytes.try_into().unwrap());

        // Vault B @ byte 352 (32 bytes)
        let vault_b_bytes = &data[352..384];
        let vault_b = Pubkey::new_from_array(vault_b_bytes.try_into().unwrap());

        // Oracle Price @ byte 16 (8 bytes, u64 in pico-USDC)
        let oracle_price_pico: u64 = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let oracle_price = oracle_price_pico as f64 / 1e6;

        // Get the mintA info
        let mint_a_account = self.client.get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = self.client.get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;

        Ok(Box::new(GoonfiPool {
            pk: *pool_address,
            oracle_price,
            mint_a,
            mint_b,
            token_program_a,
            token_program_b,
            vault_a,
            vault_b,
        }))
    }

    async fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        use crate::constants::GOONFI_PROGRAM_ID;

        // Get all accounts owned by the Goonfi program
        let accounts = self.client.get_program_accounts(&GOONFI_PROGRAM_ID)?;

        // Extract just the pubkeys
        let pubkeys: Vec<Pubkey> = accounts.into_iter().map(|(pubkey, _)| pubkey).collect();

        Ok(pubkeys)
    }
}
