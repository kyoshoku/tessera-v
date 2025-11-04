use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use crate::{
    config::Config,
    constants::{TESSERA_AUTHORITY, TESSERA_PROGRAM_ID, TESSERA_SWAP_SELECTOR},
    utils::{build_executor_instruction, get_ata, get_pubkey_from_str},
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_token::state::Mint;
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
    pub decimals_a: u8,
    pub decimals_b: u8,
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

    fn get_oracle_price(&self) -> f64 {
        self.oracle_price
    }

    fn get_decimals_a(&self) -> u8 {
        self.decimals_a
    }

    fn get_decimals_b(&self) -> u8 {
        self.decimals_b
    }

    fn get_vault_a(&self) -> Pubkey {
        self.vault_a
    }

    fn get_vault_b(&self) -> Pubkey {
        self.vault_b
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
        "Dz9mQ9NzkBcCsuGPFJ3r1bS4wgqKMHBPiVuniW8Mbonk" => {
            get_pubkey_from_str("68akEwyqPfMRV4ZrigHWSFSrZmT9GJ4WvvgFdiygsiFB").unwrap()
        }
        _ => panic!("Unsupported vault for {}", mint),
    }
}

pub struct TesseraAdapter {
    client: RpcClient,
}

impl TesseraAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    fn parse_tessera_pool(
        &self,
        pool_address: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let account = get_account(pool_address)?;
        let data = &account.data;

        let mint_a = Pubkey::new_from_array((&data[24..56]).try_into().unwrap());
        let mint_b = Pubkey::new_from_array((&data[56..88]).try_into().unwrap());

        // Get the mintA/B info
        let mint_a_account = get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;
        let decimals_a = Mint::unpack_unchecked(&mint_a_account.data)
            .unwrap()
            .decimals;

        let mint_b_account = get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;
        let decimals_b = Mint::unpack_unchecked(&mint_b_account.data)
            .unwrap()
            .decimals;

        // Oracle Price @ byte 128 (8 bytes, u64 in pico-USDC)
        // for i in 300..1200 {
        //     let oracle_price = u64::from_le_bytes(data[i..i + 8].try_into().unwrap());
        //     println!("{} - {}", i, oracle_price);
        // }

        let oracle_price_pico = u64::from_le_bytes(data[128..136].try_into().unwrap());
        let oracle_price =
            oracle_price_pico as f64 / 10e14 / 10f64.powi(decimals_a as i32 - decimals_b as i32);

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
            decimals_a,
            decimals_b,
        }))
    }
}

impl DexAdapter for TesseraAdapter {
    fn get_program_id(&self) -> Pubkey {
        TESSERA_PROGRAM_ID
    }

    fn fetch_pair_data(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        anyhow::bail!("Not implemented");
    }

    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_tessera_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        self.parse_tessera_pool(pool_address, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(1264)]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::finalized()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .client
            .get_program_accounts_with_config(&TESSERA_PROGRAM_ID, config)?;

        // Extract just the pubkeys, filtering for accounts that look like valid pools
        let pubkeys: Vec<Pubkey> = accounts
            .into_iter()
            .filter_map(|(pubkey, account)| {
                let data = account.data;
                let is_valid = *(data.get(825).unwrap()) == (0 as u8);
                // if !is_valid {
                //     return None;
                // }

                Some(pubkey)
            })
            .collect();

        Ok(pubkeys)
    }

    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        a_to_b: bool,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<TesseraPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            side: a_to_b as u8,
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
