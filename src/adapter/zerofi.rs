use crate::{
    config::Config,
    constants::{ZEROFI_PROGRAM_ID, ZEROFI_SWAP_SELECTOR},
    utils::{get_ata, get_pma_with_filter},
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
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
}

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub address: Pubkey,
    pub ata: Pubkey,
    pub oracle: Pubkey,
}

#[derive(Debug, Clone)]
pub struct ZeroFiPool {
    pub pk: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub update_authority_a: Pubkey,
    pub update_authority_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub oracle_price: f64, // in USD

    pub vault_a_state: Pubkey,
    pub vault_b_state: Pubkey,
}

impl PoolData for ZeroFiPool {
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

pub struct ZeroFiAdapter {
    client: RpcClient,
}

impl ZeroFiAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl ZeroFiAdapter {
    fn parse_zerofi_pool(
        &self,
        pool_address: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let pool_account = get_account(pool_address)?;
        let data = &pool_account.data;

        let update_authority_a = Pubkey::new_from_array((&data[8..40]).try_into().unwrap());
        let update_authority_b = Pubkey::new_from_array((&data[40..72]).try_into().unwrap());

        let mint_a = Pubkey::new_from_array((&data[72..104]).try_into().unwrap());
        let mint_b = Pubkey::new_from_array((&data[104..136]).try_into().unwrap());

        let vault_a = Pubkey::new_from_array((&data[136..168]).try_into().unwrap());
        let vault_a_state = Pubkey::new_from_array((&data[168..200]).try_into().unwrap());

        let vault_b = Pubkey::new_from_array((&data[200..232]).try_into().unwrap());
        let vault_b_state = Pubkey::new_from_array((&data[232..264]).try_into().unwrap());

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

        let vault_account = get_account(&vault_a_state)?;
        let data = &vault_account.data;

        Ok(Box::new(ZeroFiPool {
            pk: *pool_address,
            update_authority_a,
            update_authority_b,
            oracle_price: 0.0,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            decimals_a,
            decimals_b,
            token_program_a,
            token_program_b,
            vault_a_state,
            vault_b_state,
        }))
    }

    pub fn get_pool_address(&self, input_mint: &Pubkey, output_mint: &Pubkey) -> Result<Pubkey> {
        let mut accounts = get_pma_with_filter(
            &self.client,
            &self.get_program_id(),
            7456,
            vec![(72, *input_mint), (104, *output_mint)],
        )?;
        if accounts.is_empty() {
            accounts = get_pma_with_filter(
                &self.client,
                &self.get_program_id(),
                7456,
                vec![(72, *output_mint), (104, *input_mint)],
            )?;
            if accounts.is_empty() {
                anyhow::bail!(
                    "No pool address found for input mint: {} and output mint: {}",
                    input_mint,
                    output_mint
                );
            }
        }

        let (pubkey, _) = &accounts[0];
        Ok(*pubkey)
    }
}

impl DexAdapter for ZeroFiAdapter {
    fn get_program_id(&self) -> Pubkey {
        ZEROFI_PROGRAM_ID
    }

    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_zerofi_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        self.parse_zerofi_pool(pool_address, &mut get_account)
    }

    fn fetch_pair_data(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        let pool_address = self.get_pool_address(input_mint, output_mint)?;
        self.parse_zerofi_pool(&pool_address, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let accounts = get_pma_with_filter(&self.client, &self.get_program_id(), 7456, vec![])?;
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
            .downcast_ref::<ZeroFiPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            amount_in,
            min_amount_out,
        };

        let mut data = Vec::with_capacity(17);
        data.extend_from_slice(&ZEROFI_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let (user_source, user_destination) = if a_to_b {
            (
                get_ata(user, &pool.mint_a, &pool.token_program_a),
                get_ata(user, &pool.mint_b, &pool.token_program_b),
            )
        } else {
            (
                get_ata(user, &pool.mint_b, &pool.token_program_b),
                get_ata(user, &pool.mint_a, &pool.token_program_a),
            )
        };

        let (vault_source, vault_destination) = if a_to_b {
            (pool.vault_a, pool.vault_b)
        } else {
            (pool.vault_b, pool.vault_a)
        };

        let (vault_source_state, vault_destination_state) = if a_to_b {
            (pool.vault_a_state, pool.vault_b_state)
        } else {
            (pool.vault_b_state, pool.vault_a_state)
        };

        let accounts = vec![
            AccountMeta::new(pool.pk, false),
            AccountMeta::new(vault_source_state, false),
            AccountMeta::new(vault_source, false),
            AccountMeta::new(vault_destination_state, false),
            AccountMeta::new(vault_destination, false),
            AccountMeta::new(user_source, false),
            AccountMeta::new(user_destination, false),
            AccountMeta::new_readonly(*user, true), // signer
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        let swap_ix = Instruction {
            program_id: ZEROFI_PROGRAM_ID,
            accounts,
            data,
        };

        // let executor_ix = build_executor_instruction(*user, swap_ix);
        Ok(vec![swap_ix])
    }
}
