use crate::{
    config::Config,
    constants::{ALPHAQ_PROGRAM_ID, ALPHAQ_SWAP_SELECTOR},
    utils::{get_ata, get_pma_with_filter, get_token_decimals},
};
use anyhow::Result;
use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::any::Any;

use super::{DexAdapter, PoolData};

use crate::utils::{make_rpc_getter, make_svm_getter, MiniAccount};
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
    pub decimals_a: u8,
    pub decimals_b: u8,

    pub market_state: Pubkey,
    pub vendor_authority: Pubkey,
    pub token_a_authority: Pubkey,
    pub token_b_authority: Pubkey,
}

impl PoolData for AlphaqPool {
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

pub struct AlphaqAdapter {
    client: RpcClient,
}

impl AlphaqAdapter {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    fn parse_alphaq_pool(
        &self,
        pool_address: &Pubkey,
        get_account: &mut dyn FnMut(&Pubkey) -> Result<MiniAccount>,
    ) -> Result<Box<dyn PoolData>> {
        let account = get_account(pool_address)?;
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

        // Get the mintA/B info
        let mint_a_account = get_account(&mint_a)?;
        let token_program_a = mint_a_account.owner;
        let decimals_a = get_token_decimals(&mint_a_account.data)?;

        let mint_b_account = get_account(&mint_b)?;
        let token_program_b = mint_b_account.owner;
        let decimals_b = get_token_decimals(&mint_b_account.data)?;

        let market_state = self.get_market_state(&pool_address);

        Ok(Box::new(AlphaqPool {
            pk: *pool_address,
            market_state,
            oracle_price,
            mint_a,
            mint_b,
            decimals_a,
            decimals_b,
            vault_a,
            vault_b,
            token_a_authority,
            token_b_authority,
            vendor_authority,
            token_program_a,
            token_program_b,
        }))
    }

    fn get_market_state(&self, pool: &Pubkey) -> Pubkey {
        match pool.to_string().as_str() {
            "Pi9nzTjPxD8DsRfRBGfKYzmefJoJM8TcXu2jyaQjSHm" => {
                pubkey!("445fd6ffBZqWYsryCgs6wcE8exaLkRsMrefAQ5UHvt8v")
            }
            "9xPhpwq6GLUkrDBNfXCbnSP9ARAMMyUQqgkrqaDW6NLV" => {
                pubkey!("H18xqLYd5uEenmiFKoXkgrTvyhnLgJMPT8sSvdZPpi3p")
            }
            "6R3LknvRLwPg7c8Cww7LKqBHRDcGioPoj29uURX9anug" => {
                pubkey!("HjRw8yeBVUCGHMUWZzF83U4x82KwsrVwaeh6CYxtGBsQ")
            }
            "hKH9LFREBm3TxTx5Ex6D1nHTKEA8ii3CWfhEkB9s21u" => {
                pubkey!("HZyb7Gv2pWTRYq8XuaeWBePQ8CDNhxigkNohZU2dLPEC")
            }
            "2YR8bXXn4tTnq8nVjvpYnBiQ7ZKjN3G16wxE8ShL3KaB" => {
                pubkey!("aXQtJ9cGr1zgLyrppKJ5BR5jv1RMjyYL5Wetd2KZNtB")
            }
            "2o4369ha3bENAhJDan8mdRNJjNo6qQ9P5KhUS4QZUVgi" => {
                pubkey!("BLtETTx81aLGdXVmqsaRBNFG6sraxK76CE55NPLW8ja4")
            }
            "5jsvKL6eKPGUAMBYcoNn9FaKwD1i9o44YeNtEEedkmeq" => {
                pubkey!("85aqCUSm4eMv5EbRErC5VD3wuSFb43H5KJuniyAYjE5P")
            }
            "61LMyNZudQFDNMRFKwkBmf5HtRf1vU8B8FgNYdrha3fq" => {
                pubkey!("FGRoxhDzPmY3ggRZqZsBTn6aLcrAePmNwH5GXLE6cok5")
            }
            "C2GdMFGp2vSZHnU76pH2ukEWxuhoJBuaA54Ftzcvv4z5" => {
                pubkey!("CyNXfwYg6kUDh4ExsHMBpiYuDoxrNxcBgLWqseBMjq9y")
            }
            "F97Kntcg8pZrCDa9csKHPNuAtGrtzqG3RBr6UdJCz8NP" => {
                pubkey!("6MyAAUujZvo2hzHpttnqE1Zn7SWi4xrr1wJfxJLGSkqC")
            }
            "FVz9gveEdRw2fqkZFLCXxpZErS4mBQvwKwQeForLUyW6" => {
                pubkey!("9gXvza14Bp6kBDdd5rWowt7NssYy36eSNvV7MzpWzLF3")
            }
            _ => Pubkey::default(),
        }
    }
}

impl DexAdapter for AlphaqAdapter {
    fn get_program_id(&self) -> Pubkey {
        ALPHAQ_PROGRAM_ID
    }

    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_rpc_getter(&self.client);
        self.parse_alphaq_pool(pool_address, &mut get_account)
    }

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>> {
        let mut get_account = make_svm_getter(svm);
        self.parse_alphaq_pool(pool_address, &mut get_account)
    }

    fn fetch_pair_data(
        &self,
        _input_mint: &Pubkey,
        _output_mint: &Pubkey,
        _config: &Config,
    ) -> Result<Box<dyn PoolData>> {
        anyhow::bail!("Not implemented");
    }

    fn get_pools(&self, _config: &Config) -> Result<Vec<Pubkey>> {
        let accounts = get_pma_with_filter(&self.client, &self.get_program_id(), 672, vec![])?;
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
            .downcast_ref::<AlphaqPool>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast pool data"))?;

        let swap_params = SwapParams {
            a_to_b: a_to_b as u8,
            amount_in,
            min_amount_out,
        };

        // Alphaq swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&ALPHAQ_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(user, &pool.mint_a, &pool.token_program_a);
        let user_ata_b = get_ata(user, &pool.mint_b, &pool.token_program_b);

        let mut accounts = vec![
            AccountMeta::new(*user, true), // signer
            AccountMeta::new_readonly(pool.pk, false),
            AccountMeta::new(pool.market_state, false),
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

        if pool.token_program_a != spl_token::ID {
            accounts.push(AccountMeta::new_readonly(pool.mint_a, false));
            accounts.push(AccountMeta::new_readonly(pool.token_program_a, false));
        }

        let swap_ix = Instruction {
            program_id: ALPHAQ_PROGRAM_ID,
            accounts,
            data,
        };
        // let executor_ix = build_executor_instruction(*user, swap_ix);

        Ok(vec![swap_ix])
    }
}
