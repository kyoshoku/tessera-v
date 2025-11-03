use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
use crate::{
    config::Config,
    constants::{SAROS_PROGRAM_ID, SAROS_SWAP_SELECTOR},
    utils::{get_ata, get_pubkey_from_str},
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_token::state::{Account as TokenAccount, Mint};
use std::any::Any;

use super::{DexAdapter, PoolData};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

#[derive(Debug, Clone)]
pub struct SarosPool {
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

    pub pool_mint: Pubkey,
    pub pool_fee: Pubkey,
    pub token_program: Pubkey,
    pub swap_authority: Pubkey,
}

impl PoolData for SarosPool {
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

pub struct SarosAdapter {
    client: RpcClient,
}

impl SarosAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }
}

fn parse_saros_pool(
    pool_address: &Pubkey,
    get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
) -> Result<Box<dyn PoolData>> {
    let pool_acc = get_account(pool_address)?;
    let data = &pool_acc.data;

    let token_program = Pubkey::new_from_array((&data[3..35]).try_into().unwrap());
    let vault_a = Pubkey::new_from_array((&data[35..67]).try_into().unwrap());
    let vault_b = Pubkey::new_from_array((&data[67..99]).try_into().unwrap());
    let pool_mint = Pubkey::new_from_array((&data[99..131]).try_into().unwrap());
    let mint_a = Pubkey::new_from_array((&data[131..163]).try_into().unwrap());
    let mint_b = Pubkey::new_from_array((&data[163..195]).try_into().unwrap());
    let pool_fee = Pubkey::new_from_array((&data[195..227]).try_into().unwrap());

    let swap_authority = super::saros_amm::get_swap_authority(pool_address.to_string());

    // Get the mintA/B info
    let mint_a_account = get_account(&mint_a)?;
    let token_program_a = mint_a_account.owner;

    let mint_b_account = get_account(&mint_b)?;
    let token_program_b = mint_b_account.owner;

    // Compute price by unpacking token accounts and mint decimals
    let va_acc = get_account(&vault_a)?;
    let vb_acc = get_account(&vault_b)?;
    let va = TokenAccount::unpack_unchecked(&va_acc.data).unwrap();
    let vb = TokenAccount::unpack_unchecked(&vb_acc.data).unwrap();

    let mint_a_acc = Mint::unpack_unchecked(&mint_a_account.data).unwrap();
    let mint_b_acc = Mint::unpack_unchecked(&mint_b_account.data).unwrap();

    let ui_a = va.amount as f64 / 10f64.powi(mint_a_acc.decimals as i32);
    let ui_b = vb.amount as f64 / 10f64.powi(mint_b_acc.decimals as i32);
    let oracle_price = ui_a / ui_b;

    Ok(Box::new(SarosPool {
        pk: *pool_address,
        oracle_price,
        mint_a,
        mint_b,
        vault_a,
        vault_b,
        pool_fee,
        pool_mint,
        swap_authority,
        token_program_a,
        token_program_b,
        token_program,
        decimals_a: mint_a_acc.decimals,
        decimals_b: mint_b_acc.decimals,
    }))
}

fn get_swap_authority(pool: String) -> Pubkey {
    match pool.as_str() {
        "2wUvdZA8ZsY714Y5wUL9fkFmupJGGwzui2N74zqJWgty" => {
            get_pubkey_from_str("3YqR5apHmVu5CgEHuRQ33EWZp5xT5yiytSWN4bALSgRW").unwrap()
        }
        "5CrZvqqh3YyPnwQgPqcmWHQRvaYxMGZMkzUy31hJ99zc" => {
            get_pubkey_from_str("FRwe19AFk7AvNxa2P3ohygSwjAh26dF7C8L9pFg2iA2T").unwrap()
        }
        "DcTmuS7NFUcJWanJhW5mdhsU8JZyiiGJc1bYPq3G1DS8" => {
            get_pubkey_from_str("51ezHJubodJ9yaFnT329LaCfQ7MYm6Kouj8TwwEWr3Sq").unwrap()
        }
        _ => panic!("Unsupported swap authority for {}", pool),
    }
}

impl DexAdapter for SarosAdapter {
    fn get_program_id(&self) -> Pubkey {
        SAROS_PROGRAM_ID
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
        parse_saros_pool(pool_address, &mut get_account)
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        // Get all accounts owned by the Saros program
        let accounts = self.client.get_program_accounts(&SAROS_PROGRAM_ID)?;

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
            .downcast_ref::<SarosPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            amount_in,
            min_amount_out,
        };

        // Saros swap data
        let mut data = Vec::with_capacity(17);
        data.extend_from_slice(&SAROS_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let (user_source, user_destination) = if a_to_b {
            (user_ata_a, user_ata_b)
        } else {
            (user_ata_b, user_ata_a)
        };

        let (vault_source, vault_destination) = if a_to_b {
            (pool.vault_a, pool.vault_b)
        } else {
            (pool.vault_b, pool.vault_a)
        };

        let accounts = vec![
            AccountMeta::new_readonly(pool.pk, false),
            AccountMeta::new_readonly(pool.swap_authority, false),
            AccountMeta::new(*user, true), // signer
            AccountMeta::new(user_source, false),
            AccountMeta::new(vault_source, false),
            AccountMeta::new(vault_destination, false),
            AccountMeta::new(user_destination, false),
            AccountMeta::new(pool.pool_mint, false),
            AccountMeta::new(pool.pool_fee, false),
            AccountMeta::new_readonly(pool.token_program, false),
        ];

        Ok(vec![Instruction {
            program_id: SAROS_PROGRAM_ID,
            accounts,
            data,
        }])
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        parse_saros_pool(pool_address, &mut get_account)
    }
}
