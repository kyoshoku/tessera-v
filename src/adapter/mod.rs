use anyhow::Result;
use litesvm::LiteSVM;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::config::Config;

/// Common interface for DEX adapters
pub trait DexAdapter {
    /// Get the program ID for this protocol
    fn get_program_id(&self) -> Pubkey;

    /// Fetch pool account data
    fn fetch_pool_data(&self, pool_address: &Pubkey, config: &Config) -> Result<Box<dyn PoolData>>;

    fn load_pool_data(&self, pool_address: &Pubkey, svm: &LiteSVM) -> Result<Box<dyn PoolData>>;

    fn fetch_pair_data(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        config: &Config,
    ) -> Result<Box<dyn PoolData>>;

    /// Get all pool accounts for the protocol
    fn get_pools(&self, config: &Config) -> Result<Vec<Pubkey>>;

    /// Build swap instructions
    fn build_swap_instructions(
        &self,
        pool_data: &dyn PoolData,
        user: &Pubkey,
        a_to_b: bool,
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
    fn get_decimals_a(&self) -> u8;
    fn get_vault_a(&self) -> Pubkey;

    /// Get mint B
    fn get_mint_b(&self) -> Pubkey;
    fn get_decimals_b(&self) -> u8;
    fn get_vault_b(&self) -> Pubkey;

    /// Get oracle price
    fn get_oracle_price(&self) -> f64;
}

pub mod alphaq;
pub mod aquifer;
pub mod goonfi;
pub mod humidifi;
pub mod obric;
pub mod saros_amm;
pub mod tessera;
