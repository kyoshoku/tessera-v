use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use crate::{
    config::Config,
    constants::{OBRIC_PROGRAM_ID, OBRIC_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Mint;
use std::any::Any;

use super::{DexAdapter, PoolData};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub x_to_y: bool,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

#[derive(Debug, Clone)]
pub struct ObricPool {
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

    pub price_feed_x: Pubkey,
    pub price_feed_y: Pubkey,
    pub protocol_fee_x: Pubkey,
    pub protocol_fee_y: Pubkey,
    pub mint_sslp_x: Pubkey,
    pub mint_sslp_y: Pubkey,
}

impl PoolData for ObricPool {
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

pub struct ObricAdapter {
    client: RpcClient,
}

impl ObricAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    fn parse_obric_pool(
        &self,
        pool_address: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let account = get_account(pool_address)?;
        let data = &account.data;

        let price_feed_x = Pubkey::new_from_array((&data[9..41]).try_into().unwrap());
        let price_feed_y = Pubkey::new_from_array((&data[41..73]).try_into().unwrap());

        let vault_a = Pubkey::new_from_array((&data[73..105]).try_into().unwrap());
        let vault_b = Pubkey::new_from_array((&data[105..137]).try_into().unwrap());

        let protocol_fee_x = Pubkey::new_from_array((&data[137..169]).try_into().unwrap());
        let protocol_fee_y = Pubkey::new_from_array((&data[169..201]).try_into().unwrap());

        let mint_a = Pubkey::new_from_array((&data[202..234]).try_into().unwrap());
        let mint_b = Pubkey::new_from_array((&data[234..266]).try_into().unwrap());

        let mint_sslp_x = Pubkey::new_from_array((&data[482..514]).try_into().unwrap());
        let mint_sslp_y = Pubkey::new_from_array((&data[514..546]).try_into().unwrap());

        // Get the mintA info
        let mint_a_account = get_account(&mint_a)?;
        let mint_a_decimals = Mint::unpack_unchecked(&mint_a_account.data)
            .unwrap()
            .decimals;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = get_account(&mint_b)?;
        let mint_b_decimals = Mint::unpack_unchecked(&mint_b_account.data)
            .unwrap()
            .decimals;
        let token_program_b = mint_b_account.owner;

        // Oracle Price @ byte 128 (8 bytes, u64 in pico-USDC)
        let oracle_x = u64::from_le_bytes(data[306..314].try_into().unwrap());
        let oracle_y = u64::from_le_bytes(data[314..322].try_into().unwrap());
        let oracle_price = (oracle_x as f64 / oracle_y as f64)
            * 10f64.powi(mint_a_decimals as i32 - mint_b_decimals as i32);

        Ok(Box::new(ObricPool {
            pk: *pool_address,
            oracle_price,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            token_program_a,
            token_program_b,
            decimals_a: mint_a_decimals,
            decimals_b: mint_b_decimals,
            price_feed_x,
            price_feed_y,
            protocol_fee_x,
            protocol_fee_y,
            mint_sslp_x,
            mint_sslp_y,
        }))
    }
}

impl DexAdapter for ObricAdapter {
    fn get_program_id(&self) -> Pubkey {
        OBRIC_PROGRAM_ID
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
        self.parse_obric_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        self.parse_obric_pool(pool_address, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        // Get all accounts owned by the Obric program
        let accounts = self.client.get_program_accounts(&OBRIC_PROGRAM_ID)?;

        // Extract just the pubkeys
        let pubkeys: Vec<Pubkey> = accounts.into_iter().map(|(pubkey, _)| pubkey).collect();

        Ok(pubkeys)
    }

    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        x_to_y: bool,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<ObricPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            x_to_y,
            amount_in,
            min_amount_out,
        };

        // Obric swap data
        let mut data = Vec::with_capacity(25);
        data.extend_from_slice(&OBRIC_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let accounts = vec![
            AccountMeta::new(pool.pk, false),
            AccountMeta::new_readonly(pool.protocol_fee_y, false),
            AccountMeta::new_readonly(pool.mint_sslp_x, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(pool.protocol_fee_x, false),
            AccountMeta::new_readonly(pool.price_feed_x, false),
            AccountMeta::new_readonly(pool.price_feed_y, false),
            AccountMeta::new(*user, true), // signer
            AccountMeta::new_readonly(spl_token::ID, false),
        ];

        Ok(vec![Instruction {
            program_id: OBRIC_PROGRAM_ID,
            accounts,
            data,
        }])
    }
}
