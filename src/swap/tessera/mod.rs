use crate::config::Config;
use crate::constants::{TESSERA_SWAP_SELECTOR, WSOL_MINT};
use crate::fetch::tessera::TesseraPool;
use crate::swap::SwapBuilder;
use crate::utils::{
    build_executor_instruction, build_unwrap_sol_instruction, build_wrap_sol_instruction, get_ata,
};
use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub side: u8,            // Protocol-specific side indicator
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

/// Tessera swap instruction builder
pub struct TesseraSwapBuilder {
    pool: TesseraPool,
    config: Config,
    user: Pubkey,
}

impl TesseraSwapBuilder {
    pub fn new(pool: TesseraPool, user: Pubkey, config: Config) -> Self {
        Self { pool, user, config }
    }
}

impl SwapBuilder for TesseraSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        self.config.get_tessera_program_id()
    }

    /// Build swap instruction with automatic side detection based on input token
    fn build_swap(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        wrap_sol: bool,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];

        let wsol_ata = get_associated_token_address_with_program_id(
            &self.user,
            &Pubkey::from_str(WSOL_MINT).unwrap(),
            &spl_token::ID,
        );
        if wrap_sol {
            instructions.extend(build_wrap_sol_instruction(&self.user, &wsol_ata, amount_in));
        }

        // Build the swap instruction
        let swap_ix = self
            .build_swap_instruction(input_mint, amount_in, min_amount_out)
            .unwrap();
        let executor_ix =
            build_executor_instruction(self.user, self.get_program_id(), swap_ix.accounts, swap_ix.data);
        instructions.push(executor_ix);

        if wrap_sol {
            instructions.extend(build_unwrap_sol_instruction(&self.user, &wsol_ata));
        }

        Ok(instructions)
    }
}

impl TesseraSwapBuilder {
    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let side = input_mint.eq(&self.pool.mint_a) as u8;
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
