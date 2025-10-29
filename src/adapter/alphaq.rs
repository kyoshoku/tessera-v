use crate::{
    config::Config,
    constants::{ALPHAQ_PROGRAM_ID, ALPHAQ_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::RpcFilterType,
};
use solana_program::pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::any::Any;

use super::{DexAdapter, PoolData};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub a_to_b: u8,          // Protocol-specific side indicator
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

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

pub struct AlphaqAdapter {
    client: RpcClient,
}

impl AlphaqAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl DexAdapter for AlphaqAdapter {
    fn fetch_pool_data(
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
        let oracle_price_pico: u64 = u64::from_le_bytes(data[424..432].try_into().unwrap());
        let oracle_price = oracle_price_pico as f64 / 1e10;

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

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(672)]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::finalized()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .client
            .get_program_accounts_with_config(&ALPHAQ_PROGRAM_ID, config)?;

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
            .downcast_ref::<AlphaqPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let a_to_b = input_token.eq(&pool.mint_a) as u8;
        let swap_params = SwapParams {
            a_to_b,
            amount_in,
            min_amount_out,
        };

        // Alphaq swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&ALPHAQ_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let market_state = pubkey!("HZyb7Gv2pWTRYq8XuaeWBePQ8CDNhxigkNohZU2dLPEC");

        let accounts = vec![
            AccountMeta::new(*user, true), // signer
            AccountMeta::new_readonly(pool.pk, false),
            AccountMeta::new(market_state, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(pool.token_a_authority, false),
            AccountMeta::new(pool.token_b_authority, false),
            AccountMeta::new(pool.vendor_authority, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        Ok(vec![Instruction {
            program_id: ALPHAQ_PROGRAM_ID,
            accounts,
            data,
        }])
    }
}
