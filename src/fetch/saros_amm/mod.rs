use crate::{
    config::Config,
    fetch::{PoolData, PoolFetcher},
    utils::get_pubkey_from_str,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;


#[derive(Debug, Clone)]
pub struct SarosPool {
    pub pk: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub oracle_price: f64, // in USD

    pub pool_mint: Pubkey,
    pub pool_fee: Pubkey,
    pub token_program: Pubkey,
    pub swap_authority: Pubkey,
}

impl PoolData for SarosPool {
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

pub struct SarosPoolFetcher {
    client: RpcClient,
}

impl SarosPoolFetcher {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

fn get_swap_authority(pool: String) -> Pubkey {
    match pool.as_str() {
        "2wUvdZA8ZsY714Y5wUL9fkFmupJGGwzui2N74zqJWgty" => {
            get_pubkey_from_str("3YqR5apHmVu5CgEHuRQ33EWZp5xT5yiytSWN4bALSgRW").unwrap()
        }
        "5CrZvqqh3YyPnwQgPqcmWHQRvaYxMGZMkzUy31hJ99zc" => {
            get_pubkey_from_str("FRwe19AFk7AvNxa2P3ohygSwjAh26dF7C8L9pFg2iA2T").unwrap()
        }
        "DcTmuS7NFUcJWanJhW5mdhsU8JZyiiGJc1bYPq3G1DS8" => {
            get_pubkey_from_str("51ezHJubodJ9yaFnT329LaCfQ7MYm6Kouj8TwwEWr3Sq").unwrap()
        }
        _ => panic!("Unsupported swap authority for {}", pool),
    }
}

impl PoolFetcher for SarosPoolFetcher {
    async fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let account = self.client.get_account(pool_address)?;
        let data = &account.data;

        let token_program = Pubkey::new_from_array((&data[3..35]).try_into().unwrap());
        let vault_a = Pubkey::new_from_array((&data[35..67]).try_into().unwrap());
        let vault_b = Pubkey::new_from_array((&data[67..99]).try_into().unwrap());
        let pool_mint = Pubkey::new_from_array((&data[99..131]).try_into().unwrap());
        let mint_a = Pubkey::new_from_array((&data[131..163]).try_into().unwrap());
        let mint_b = Pubkey::new_from_array((&data[163..195]).try_into().unwrap());
        let pool_fee = Pubkey::new_from_array((&data[195..227]).try_into().unwrap());

        let swap_authority = get_swap_authority(pool_address.to_string());

        // Get the mintA/B info
        let mint_a_account = self.client.get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = self.client.get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;

        // Saros AMM, so no oracle price & calcualte from vault (should apply AMM rule)
        let vault_a_amount = self.client.get_token_account_balance(&vault_a)?;
        let vault_b_amount = self.client.get_token_account_balance(&vault_b)?;
        let oracle_price = vault_a_amount.ui_amount.unwrap_or_default()
            / vault_b_amount.ui_amount.unwrap_or_default();

        Ok(Box::new(SarosPool {
            pk: *pool_address,
            oracle_price,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            pool_fee,
            pool_mint,
            swap_authority,
            token_program_a,
            token_program_b,
            token_program,
        }))
    }
}
