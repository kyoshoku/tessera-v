use crate::config::Config;
use crate::constants::TESSERA_SWAP_SELECTOR;
use crate::fetch::tessera::TesseraPoolData;
use crate::swap::{SwapBuilder, SwapParams};
use crate::utils::get_ata;
use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Tessera swap instruction builder
pub struct TesseraSwapBuilder {
    pool: TesseraPoolData,
    config: Config,
    user: Pubkey,
}

impl TesseraSwapBuilder {
    pub fn new(pool: TesseraPoolData, user: Pubkey, config: Config) -> Self {
        Self { pool, user, config }
    }
}

impl SwapBuilder for TesseraSwapBuilder {
    /// Build swap instruction with automatic side detection based on input token
    fn build_swap(
        &self,
        input_token: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let side = if input_token.eq(&self.pool.mint_a) {
            1 // base → quote
        } else {
            0
        };

        self.build_swap_instruction(side, amount_in, min_amount_out)
    }
}

impl TesseraSwapBuilder {
    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        side: u8,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let swap_params = SwapParams {
            side,
            amount_in,
            min_amount_out,
        };

        // Tessera swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&TESSERA_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(&self.user, &self.pool.mint_a, &self.pool.token_program_a);
        let user_ata_b = get_ata(&self.user, &self.pool.mint_b, &self.pool.token_program_b);

        let accounts = vec![
            AccountMeta::new_readonly(self.config.get_tessera_authority(), false),
            AccountMeta::new(self.pool.pk, false),
            AccountMeta::new(self.user, true), // signer
            AccountMeta::new(self.pool.vault_a, false),
            AccountMeta::new(self.pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(self.pool.mint_a, false),
            AccountMeta::new_readonly(self.pool.mint_b, false),
            AccountMeta::new_readonly(self.pool.token_program_a, false),
            AccountMeta::new_readonly(self.pool.token_program_b, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        Ok(Instruction {
            program_id: self.config.get_tessera_program_id(),
            accounts,
            data,
        })
    }
}
