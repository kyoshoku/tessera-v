use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::{account::Account, instruction::AccountMeta, pubkey::Pubkey};
use std::collections::HashMap;

pub type AccountMap = HashMap<Pubkey, Account>;

#[derive(Debug)]
pub struct QuoteParams {
    pub amount: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
}

pub struct SwapAndAccountMetas {
    pub swap: Swap,
    pub account_metas: Vec<AccountMeta>,
}

#[derive(Default)]
pub struct SwapParams {
    pub amount_in: u64,
    pub amount_out: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_token_account: Pubkey,
    pub output_token_account: Pubkey,
    /// This can be the user or the program authority over the input_token_account.
    pub token_transfer_authority: Pubkey,
}

/// This enum is a jupiter backend enum and does not map 1:1 to the onchain aggregator Swap enum
#[derive(Clone, PartialEq, Debug)]
pub enum Swap {
    AQUIFER,
}

//#[async_trait]
pub trait Pair {
    /// A human readable label of the underlying DEX
    fn label(&self) -> String;
    /// Dex swap program pubkey
    fn program_id(&self) -> Pubkey;
    /// The pool state or market state address
    fn key(&self) -> Pubkey;
    /// The mint that can be traded
    fn get_base_mint(&self) -> Pubkey;
    /// The mint that can be traded
    fn get_quote_mint(&self) -> Pubkey;
    /// The mints that can be traded
    fn get_mints(&self) -> Vec<Pubkey>;
    /// The accounts necessary to produce a quote
    fn get_accounts_to_update(&self) -> Vec<Pubkey>;
    /// Picks necessary accounts to update it's internal state
    /// Heavy deserialization and precomputation caching should be done in this function
    fn update(&mut self, account_map: &AccountMap) -> Result<()>;

    fn quote(&self, quote_params: &QuoteParams) -> Result<u64>;

    /// Indicates which Swap has to be performed along with all the necessary account metas
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas>;

    /// Provides a shortcut to establish if the AMM can be used for trading
    /// If the market is active at all
    fn is_active(&self) -> bool {
        true
    }
}
