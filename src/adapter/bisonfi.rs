use crate::{
    config::Config,
    constants::{BISONFI_PROGRAM_ID, BISONFI_SWAP_SELECTOR},
    utils::{get_ata, get_pma_with_filter},
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::{rpc_client::RpcClient};
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::state::Mint;
use std::any::Any;

use super::{DexAdapter, PoolData};

use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
    pub a_to_b: u8,          // Protocol-specific side indicator
}

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub address: Pubkey,
    pub ata: Pubkey,
    pub oracle: Pubkey,
}

#[derive(Debug, Clone)]
pub struct BisonfiPool {
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

impl PoolData for BisonfiPool {
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

    fn get_decimals_a(&self) -> u8 {
        self.decimals_a
    }

    fn get_decimals_b(&self) -> u8 {
        self.decimals_b
    }

    fn get_oracle_price(&self) -> f64 {
        self.oracle_price
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

pub struct BisonfiAdapter {
    client: RpcClient,
}

impl BisonfiAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    fn parse_bisonfi_pool(
        &self,
        pool_address: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let account = get_account(pool_address)?;
        let data = &account.data;


    let vault_a = Pubkey::new_from_array((&data[120..152]).try_into().unwrap());
    let vault_b = Pubkey::new_from_array((&data[152..184]).try_into().unwrap());
    let mint_a = Pubkey::new_from_array((&data[184..216]).try_into().unwrap());
    let mint_b = Pubkey::new_from_array((&data[216..248]).try_into().unwrap());

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

    Ok(Box::new(BisonfiPool {
        pk: *pool_address,
        oracle_price: 0.0,
        mint_a,
        mint_b,
        vault_a,
        vault_b,
        decimals_a,
        decimals_b,
        token_program_a,
        token_program_b,
    }))
    }

    fn parse_bisonfi_pair(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let mint_a_account = get_account(&input_mint)?;
        let token_program_a = mint_a_account.owner;
        let decimals_a = Mint::unpack_unchecked(&mint_a_account.data)
            .unwrap()
            .decimals;

        let mint_b_account = get_account(&output_mint)?;
        let token_program_b = mint_b_account.owner;
        let decimals_b = Mint::unpack_unchecked(&mint_b_account.data)
            .unwrap()
            .decimals;

        let vault_a = get_associated_token_address_with_program_id(input_mint, &token_program_a, &BISONFI_PROGRAM_ID);
        let vault_b = get_associated_token_address_with_program_id(output_mint, &token_program_b, &BISONFI_PROGRAM_ID);

        Ok(Box::new(BisonfiPool {
            pk: Pubkey::default(),
            oracle_price: 0.0,
            mint_a: *input_mint,
            mint_b: *output_mint,
            decimals_a,
            decimals_b,
            token_program_a,
            token_program_b,
            vault_a,
            vault_b
        }))
    }
}

impl DexAdapter for BisonfiAdapter {
    fn get_program_id(&self) -> Pubkey {
        BISONFI_PROGRAM_ID
    }

    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_bisonfi_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        self.parse_bisonfi_pool(pool_address, &mut get_account)
    }

    fn fetch_pair_data(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_bisonfi_pair(input_mint, output_mint, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let accounts = get_pma_with_filter(&self.client, &self.get_program_id(), 2048, vec![])?;
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
            .downcast_ref::<BisonfiPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            amount_in,
            min_amount_out,
            a_to_b: !a_to_b as u8,
        };

        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&BISONFI_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let accounts = vec![
            AccountMeta::new(*user, true), // signer
            AccountMeta::new(pool.pk, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(pool.token_program_a, false),
            AccountMeta::new_readonly(pool.token_program_b, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        Ok(vec![Instruction {
            program_id: BISONFI_PROGRAM_ID,
            accounts,
            data,
        }])
    }
}
