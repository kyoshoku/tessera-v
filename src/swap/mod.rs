use anyhow::Result;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Common interface for building swap instructions across different protocols
pub trait SwapBuilder {
    /// Build swap instruction
    fn build_swap(
        &self,
        input_token: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction>;
}

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub side: u8,            // Protocol-specific side indicator
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

pub mod tessera;
