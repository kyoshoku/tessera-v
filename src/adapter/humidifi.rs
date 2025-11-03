use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use crate::{
    config::Config,
    constants::{HUMIDIFI_PROGRAM_ID, HUMIDIFI_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
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
    pub swap_id: u64,
    pub amount_in: u64,
    pub is_base_to_quote: u8,
    pub padding: [u8; 7],
}

#[derive(Debug, Clone)]
pub struct HumidifiPool {
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

impl PoolData for HumidifiPool {
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

pub struct HumidifiAdapter {
    client: RpcClient,
}

impl HumidifiAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

fn parse_humidifi_pool(
    pool_address: &Pubkey,
    get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
) -> Result<Box<dyn PoolData>> {
    let account = get_account(pool_address)?;
    let data = &account.data;

    let mint_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let mint_b = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
    let vault_a = Pubkey::from_str("AzzKL5BpnX9UXfJRyiGbAwyFvw93vwCTBvW56tJebsqd").unwrap();
    let vault_b = Pubkey::from_str("EmyNiSYtMYZsBCVYpzLwNduQFqCpDGR8qM1h2CoLpWnM").unwrap();

    // Oracle Price @ byte 16 (8 bytes, u64 in pico-USDC)
    // for i in 0..800 {
    //     println!("{} - {}", i, u64::from_le_bytes(data[i..i + 8].try_into().unwrap()));
    // }

    let oracle_price_pico: u64 = u64::from_le_bytes(data[324..332].try_into().unwrap());
    let oracle_price = oracle_price_pico as f64 / 1e12;

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

    Ok(Box::new(HumidifiPool {
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

impl DexAdapter for HumidifiAdapter {
    fn get_program_id(&self) -> Pubkey {
        HUMIDIFI_PROGRAM_ID
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
        parse_humidifi_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        parse_humidifi_pool(pool_address, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(1728)]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::finalized()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .client
            .get_program_accounts_with_config(&self.get_program_id(), config)?;

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
        _min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<HumidifiPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            swap_id: 0,
            amount_in,
            is_base_to_quote: a_to_b as u8,
            padding: [0; 7],
        };

        // Humidifi swap data
        let mut data = Vec::with_capacity(25);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);
        data.extend_from_slice(&HUMIDIFI_SWAP_SELECTOR);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let accounts = vec![
            AccountMeta::new(*user, true), // signer
            AccountMeta::new(pool.pk, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(solana_sdk::clock::sysvar::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        Ok(vec![Instruction {
            program_id: self.get_program_id(),
            accounts,
            data,
        }])
    }
}
