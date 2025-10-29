use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::config::Config;

/// Common interface for fetching pool data from different protocols
pub trait PoolFetcher {
    /// Fetch pool account data
    async fn fetch_pool_data(
        &self,
        pool_address: &Pubkey,
        config: &Config,
    ) -> Result<Box<dyn PoolData>>;
    
    /// Get all pool accounts for the protocol
    async fn get_pools(&self, config: &Config) -> Result<Vec<Pubkey>>;
}

/// Common interface for pool data across different protocols
pub trait PoolData: Any {
    /// Display formatted pool data
    fn display(&self);

    /// Get reference to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

pub mod goonfi;
pub mod obric;
pub mod saros_amm;
pub mod tessera;
pub mod alphaq;
