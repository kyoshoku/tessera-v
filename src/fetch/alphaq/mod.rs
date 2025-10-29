use crate::{
    config::Config,
    fetch::{PoolData, PoolFetcher},
    utils::get_pubkey_from_str,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct AlphaqPool {
    pub pk: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub oracle_price: f64, // in USD

    pub vendor_authority: Pubkey,
    pub token_a_authority: Pubkey,
    pub token_b_authority: Pubkey,
}

impl PoolData for AlphaqPool {
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

pub struct AlphaqPoolFetcher {
    client: RpcClient,
}

impl AlphaqPoolFetcher {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl PoolFetcher for AlphaqPoolFetcher {
    async fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let account = self.client.get_account(pool_address)?;
        let data = &account.data;

        let vault_a = Pubkey::new_from_array((&data[112..144]).try_into().unwrap());
        let vault_b = Pubkey::new_from_array((&data[144..176]).try_into().unwrap());

        let token_a_authority = Pubkey::new_from_array((&data[176..208]).try_into().unwrap());
        let token_b_authority = Pubkey::new_from_array((&data[208..240]).try_into().unwrap());

        let mint_a = Pubkey::new_from_array((&data[240..272]).try_into().unwrap());
        let mint_b = Pubkey::new_from_array((&data[272..304]).try_into().unwrap());

        let vendor_authority = Pubkey::new_from_array((&data[304..336]).try_into().unwrap());

        // Oracle Price @ byte 16 (8 bytes, u64 in pico-USDC)
        let oracle_price_pico: u64 = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let oracle_price = oracle_price_pico as f64 / 1e6;

        // Get the mintA info
        let mint_a_account = self.client.get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = self.client.get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;

        Ok(Box::new(AlphaqPool {
            pk: *pool_address,
            oracle_price,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            token_a_authority,
            token_b_authority,
            vendor_authority,
            token_program_a,
            token_program_b,
        }))
    }
}
