use crate::{
    config::Config,
    constants::{GOONFI_PROGRAM_ID, GOONFI_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::{any::Any, str::FromStr};

use super::{DexAdapter, PoolData};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub is_user_bid: bool,
    pub bump: u8,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_mint_a(&self) -> Pubkey {
        self.mint_a
    }

    fn get_mint_b(&self) -> Pubkey {
        self.mint_b
    }
}

pub struct GoonfiAdapter {
    client: RpcClient,
}

impl GoonfiAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl DexAdapter for GoonfiAdapter {
    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let account = self.client.get_account(pool_address)?;
        let data = &account.data;

        // Parse according to Goonfi structure
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

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        // Get all accounts owned by the Goonfi program
        let accounts = self.client.get_program_accounts(&GOONFI_PROGRAM_ID)?;

        // Extract just the pubkeys
        let pubkeys: Vec<Pubkey> = accounts.into_iter().map(|(pubkey, _)| pubkey).collect();

        Ok(pubkeys)
    }

    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        input_token: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<GoonfiPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let is_user_bid = input_token.eq(&pool.mint_b) as bool;
        let swap_params = SwapParams {
            is_user_bid,
            bump: 0xff,
            amount_in,
            min_amount_out,
        };

        // Goonfi swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&GOONFI_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);
        let blacklist = Pubkey::from_str("EMnJF7cbUBF3anzozC1JhZtj9qPtbAqnJtpMuGzpEnKB").unwrap();

        let accounts = vec![
            AccountMeta::new(*user, true), // signer
            AccountMeta::new(pool.pk, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new_readonly(blacklist, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
            AccountMeta::new_readonly(pool.token_program_a, false),
        ];

        Ok(vec![Instruction {
            program_id: GOONFI_PROGRAM_ID,
            accounts,
            data,
        }])
    }
}
