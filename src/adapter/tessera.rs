use crate::{
    config::Config,
    constants::{TESSERA_AUTHORITY, TESSERA_PROGRAM_ID, TESSERA_SWAP_SELECTOR},
    utils::{build_executor_instruction, get_ata, get_pubkey_from_str},
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::any::Any;

use super::{DexAdapter, PoolData};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub side: u8,            // Protocol-specific side indicator
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

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

pub struct TesseraAdapter {
    client: RpcClient,
}

impl TesseraAdapter {
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

impl DexAdapter for TesseraAdapter {
    fn fetch_pool_data(
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

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        // Get all accounts owned by the Tessera program
        let accounts = self.client.get_program_accounts(&TESSERA_PROGRAM_ID)?;
        
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
            .downcast_ref::<TesseraPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let side = input_token.eq(&pool.mint_a) as u8;
        let swap_params = SwapParams {
            side,
            amount_in,
            min_amount_out,
        };

        // Tessera swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&TESSERA_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let accounts = vec![
            AccountMeta::new_readonly(TESSERA_AUTHORITY, false),
            AccountMeta::new(pool.pk, false),
            AccountMeta::new(*user, true), // signer
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(pool.mint_a, false),
            AccountMeta::new_readonly(pool.mint_b, false),
            AccountMeta::new_readonly(pool.token_program_a, false),
            AccountMeta::new_readonly(pool.token_program_b, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        let swap_ix = Instruction {
            program_id: TESSERA_PROGRAM_ID,
            accounts,
            data,
        };

        let executor_ix = build_executor_instruction(*user, swap_ix);

        Ok(vec![executor_ix])
    }
}
