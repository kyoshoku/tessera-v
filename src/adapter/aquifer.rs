use crate::{
    config::Config,
    constants::{
        AQUIFER_POOL_AUTHORITY, AQUIFER_POOL_STATE, AQUIFER_PROGRAM_ID, AQUIFER_SWAP_SELECTOR,
    },
    utils::get_ata,
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_program::program_pack::Pack;
use solana_program::pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
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
    // pub a_to_b: u8,     // Protocol-specific side indicator
    pub amount_in: u64, // Input amount
}

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub address: Pubkey,
    pub ata: Pubkey,
    pub oracle: Pubkey,
}

#[derive(Debug, Clone)]
pub struct AquiferPool {
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

    pub oracle_a: Pubkey,
    pub oracle_b: Pubkey,
    pub vault_ata_a: Pubkey,
    pub vault_ata_b: Pubkey,
}

impl PoolData for AquiferPool {
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
}

pub struct AquiferAdapter {
    client: RpcClient,
}

impl AquiferAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

fn parse_aqufier_pool(
    pool_address: &Pubkey,
    get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
) -> Result<Box<dyn PoolData>> {
    anyhow::bail!("Not implemented");
}

impl AquiferAdapter {
    fn parse_aqufier_pair(
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

        let vault_a_info = self.get_vault_info(*input_mint)?;
        let vault_b_info = self.get_vault_info(*output_mint)?;

        Ok(Box::new(AquiferPool {
            pk: Pubkey::default(),
            oracle_price: 0.0,
            mint_a: *input_mint,
            mint_b: *output_mint,
            decimals_a,
            decimals_b,
            token_program_a,
            token_program_b,
            vault_a: vault_a_info.address,
            vault_b: vault_b_info.address,
            oracle_a: vault_a_info.oracle,
            oracle_b: vault_b_info.oracle,
            vault_ata_a: vault_a_info.ata,
            vault_ata_b: vault_b_info.ata,
        }))
    }

    pub fn get_vault_info(&self, mint: Pubkey) -> Result<VaultInfo> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::DataSize(1056),
                RpcFilterType::Memcmp(Memcmp::new(
                    952,
                    MemcmpEncodedBytes::Base58(mint.to_string()),
                )),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::finalized()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .client
            .get_program_accounts_with_config(&AQUIFER_PROGRAM_ID, config)?;
        if accounts.is_empty() {
            anyhow::bail!("No vault info found for mint: {}", mint);
        }

        let (pubkey, account) = &accounts[0];
        let oracle = Pubkey::new_from_array((&account.data[920..952]).try_into().unwrap());
        let ata = Pubkey::new_from_array((&account.data[984..1016]).try_into().unwrap());

        Ok(VaultInfo {
            address: *pubkey,
            oracle,
            ata,
        })
    }
}

impl DexAdapter for AquiferAdapter {
    fn get_program_id(&self) -> Pubkey {
        AQUIFER_PROGRAM_ID
    }

    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        parse_aqufier_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        parse_aqufier_pool(pool_address, &mut get_account)
    }

    fn fetch_pair_data(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_aqufier_pair(input_mint, output_mint, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(1056)]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::finalized()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .client
            .get_program_accounts_with_config(&AQUIFER_PROGRAM_ID, config)?;

        // Extract just the pubkeys
        let pubkeys: Vec<Pubkey> = accounts.into_iter().map(|(pubkey, _)| pubkey).collect();

        Ok(pubkeys)
    }

    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        _a_to_b: bool,
        amount_in: u64,
        _min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<AquiferPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            // a_to_b: a_to_b as u8,
            amount_in,
        };

        let mut data = Vec::with_capacity(9);
        data.extend_from_slice(&AQUIFER_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let accounts = vec![
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
            AccountMeta::new(*user, true), // signer
            AccountMeta::new_readonly(pool.token_program_b, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(pool.mint_b, false),
            AccountMeta::new_readonly(pool.token_program_a, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new_readonly(pool.mint_a, false),
            AccountMeta::new_readonly(AQUIFER_POOL_AUTHORITY, false),
            AccountMeta::new(AQUIFER_POOL_STATE, false),
            AccountMeta::new_readonly(pool.oracle_b, false),
            AccountMeta::new_readonly(pool.oracle_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(pool.vault_ata_b, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_ata_a, false),
        ];

        Ok(vec![Instruction {
            program_id: AQUIFER_PROGRAM_ID,
            accounts,
            data,
        }])
    }
}
