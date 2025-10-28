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
        wrap_sol: bool,
    ) -> Result<Vec<Instruction>>;

    fn get_program_id(&self) -> Pubkey;
}

pub mod goonfi;
pub mod tessera;
