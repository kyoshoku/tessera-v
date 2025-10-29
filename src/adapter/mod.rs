use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::config::Config;

/// Common interface for DEX adapters
pub trait DexAdapter {
    /// Fetch pool account data
    fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        config: &Config,
    ) -> Result<Box<dyn PoolData>>;

    /// Get all pool accounts for the protocol
    fn get_pools(&self, config: &Config) -> Result<Vec<Pubkey>>;

    /// Build swap instructions
    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        input_token: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<solana_sdk::instruction::Instruction>>;
}

/// Common interface for pool data across different protocols
pub trait PoolData: Any {
    /// Display formatted pool data
    fn display(&self);

    /// Get reference to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get mint A
    fn get_mint_a(&self) -> Pubkey;

    /// Get mint B
    fn get_mint_b(&self) -> Pubkey;
}

pub mod tessera;
pub mod alphaq;
pub mod goonfi;
pub mod obric;
pub mod saros_amm;
