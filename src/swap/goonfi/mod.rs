use crate::config::Config;
use crate::constants::GOONFI_SWAP_SELECTOR;
use crate::fetch::goonfi::GoonfiPool;
use crate::swap::SwapBuilder;
use crate::utils::get_ata;
use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub is_user_bid: bool,
    pub bump: u8,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

/// Goonfi swap instruction builder
pub struct GoonfiSwapBuilder {
    pool: GoonfiPool,
    config: Config,
    user: Pubkey,
}

impl GoonfiSwapBuilder {
    pub fn new(pool: GoonfiPool, user: Pubkey, config: Config) -> Self {
        Self { pool, user, config }
    }
}

impl SwapBuilder for GoonfiSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        self.config.goonfi_program_id
    }

    /// Build swap instruction with automatic side detection based on input token
    fn build_swap(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        instructions.push(
            self.build_swap_instruction(input_mint, amount_in, min_amount_out)
                .unwrap(),
        );

        Ok(instructions)
    }
}

impl GoonfiSwapBuilder {
    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let is_user_bid = input_mint.eq(&self.pool.mint_b) as bool;
        let swap_params = SwapParams {
            is_user_bid,
            bump: 0xff,
            amount_in,
            min_amount_out,
        };

        // Tessera swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&GOONFI_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(&self.user, &self.pool.mint_a, &self.pool.token_program_a);
        let user_ata_b = get_ata(&self.user, &self.pool.mint_b, &self.pool.token_program_b);
        let blacklist = Pubkey::from_str("EMnJF7cbUBF3anzozC1JhZtj9qPtbAqnJtpMuGzpEnKB").unwrap();

        let accounts = vec![
            AccountMeta::new(self.user, true), // signer
            AccountMeta::new(self.pool.pk, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new(self.pool.vault_a, false),
            AccountMeta::new(self.pool.vault_b, false),
            AccountMeta::new_readonly(blacklist, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
            AccountMeta::new_readonly(self.pool.token_program_a, false),
        ];

        Ok(Instruction {
            program_id: self.get_program_id(),
            accounts,
            data,
        })
    }
}
