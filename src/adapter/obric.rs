use crate::{
    config::Config,
    constants::{OBRIC_PROGRAM_ID, OBRIC_SWAP_SELECTOR},
    utils::get_ata,
};
use anyhow::Result;
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
}

pub struct ObricAdapter {
    client: RpcClient,
}

impl ObricAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

impl DexAdapter for ObricAdapter {
    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let account = self.client.get_account(pool_address)?;
        let data = &account.data;

        // Parse according to Obric structure
        // Discriminator @ bytes 0-7
        let _discriminator = data[0..8].to_vec();

        let price_feed_x_bytes = &data[9..41];
        let price_feed_x = Pubkey::new_from_array(price_feed_x_bytes.try_into().unwrap());
        let price_feed_y_bytes = &data[41..73];
        let price_feed_y = Pubkey::new_from_array(price_feed_y_bytes.try_into().unwrap());

        let vault_a_bytes = &data[73..105];
        let vault_a = Pubkey::new_from_array(vault_a_bytes.try_into().unwrap());
        let vault_b_bytes = &data[105..137];
        let vault_b = Pubkey::new_from_array(vault_b_bytes.try_into().unwrap());

        let protocol_fee_x_bytes = &data[137..169];
        let protocol_fee_x = Pubkey::new_from_array(protocol_fee_x_bytes.try_into().unwrap());
        let protocol_fee_y_bytes = &data[169..201];
        let protocol_fee_y = Pubkey::new_from_array(protocol_fee_y_bytes.try_into().unwrap());

        let mint_a_bytes = &data[202..234];
        let mint_a = Pubkey::new_from_array(mint_a_bytes.try_into().unwrap());
        let mint_b_bytes = &data[234..266];
        let mint_b = Pubkey::new_from_array(mint_b_bytes.try_into().unwrap());

        let mint_sslp_x_bytes = &data[482..514];
        let mint_sslp_x = Pubkey::new_from_array(mint_sslp_x_bytes.try_into().unwrap());
        let mint_sslp_y_bytes = &data[514..546];
        let mint_sslp_y = Pubkey::new_from_array(mint_sslp_y_bytes.try_into().unwrap());

        // Get the mintA info
        let mint_a_account = self.client.get_account(&mint_a)?;
        let mint_a_decimals = Mint::unpack_unchecked(&mint_a_account.data)
            .unwrap()
            .decimals;
        let token_program_a = mint_a_account.owner;

        let mint_b_account = self.client.get_account(&mint_b)?;
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
            price_feed_x,
            price_feed_y,
            protocol_fee_x,
            protocol_fee_y,
            mint_sslp_x,
            mint_sslp_y,
        }))
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
        input_token: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let pool = pool_data
            .as_any()
            .downcast_ref::<ObricPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let x_to_y = input_token.eq(&pool.mint_a) as bool;
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
