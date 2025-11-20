use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use crate::{
    config::Config,
    constants::{GOONFI_PROGRAM_ID, GOONFI_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_token::state::Mint;
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
    pub decimals_a: u8,
    pub decimals_b: u8,
}

impl PoolData for GoonfiPool {
    fn display(&self) {
        println!("Pool: {}", self.pk);
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

    fn get_token_program_a(&self) -> Pubkey {
        self.token_program_a
    }

    fn get_token_program_b(&self) -> Pubkey {
        self.token_program_b
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

fn parse_goonfi_pool(
    pool_address: &Pubkey,
    get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
) -> Result<Box<dyn PoolData>> {
    let account = get_account(pool_address)?;
    let data = &account.data;

    let mint_a = Pubkey::new_from_array((&data[256..288]).try_into().unwrap());
    let mint_b = Pubkey::new_from_array((&data[288..320]).try_into().unwrap());
    let vault_a = Pubkey::new_from_array((&data[320..352]).try_into().unwrap());
    let vault_b = Pubkey::new_from_array((&data[352..384]).try_into().unwrap());

    // Oracle Price @ byte 16 (8 bytes, u64 in pico-USDC)
    let oracle_price_pico: u64 = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let oracle_price = oracle_price_pico as f64 / 1e6;

    // Get the mintA info
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

    Ok(Box::new(GoonfiPool {
        pk: *pool_address,
        oracle_price,
        mint_a,
        mint_b,
        token_program_a,
        token_program_b,
        vault_a,
        vault_b,
        decimals_a,
        decimals_b,
    }))
}

impl DexAdapter for GoonfiAdapter {
    fn get_program_id(&self) -> Pubkey {
        GOONFI_PROGRAM_ID
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
        parse_goonfi_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        parse_goonfi_pool(pool_address, &mut get_account)
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
        a_to_b: bool,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<GoonfiPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let is_user_bid = !a_to_b;
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
