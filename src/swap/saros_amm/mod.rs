use crate::constants::SAROS_SWAP_SELECTOR;
use crate::fetch::saros_amm::SarosPool;
use crate::swap::SwapBuilder;
use crate::utils::get_ata;
use crate::{config::Config, constants::SAROS_PROGRAM_ID};
use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

/// Saros swap instruction builder
pub struct SarosSwapBuilder {
    pool: SarosPool,
    config: Config,
    user: Pubkey,
}

impl SarosSwapBuilder {
    pub fn new(pool: SarosPool, user: Pubkey, config: Config) -> Self {
        Self { pool, user, config }
    }
}

impl SwapBuilder for SarosSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        SAROS_PROGRAM_ID
    }

    /// Build swap instruction with automatic side detection based on input token
    fn build_swap(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        let swap_ix = self
            .build_swap_instruction(input_mint, amount_in, min_amount_out)
            .unwrap();
        instructions.push(swap_ix);

        Ok(instructions)
    }
}

impl SarosSwapBuilder {
    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let swap_params = SwapParams {
            amount_in,
            min_amount_out,
        };

        // Tessera swap data
        let mut data = Vec::with_capacity(17);
        data.extend_from_slice(&SAROS_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let is_a_to_b = input_mint.eq(&self.pool.mint_a);

        let user_ata_a = get_ata(&self.user, &self.pool.mint_a, &self.pool.token_program_a);
        let user_ata_b = get_ata(&self.user, &self.pool.mint_b, &self.pool.token_program_b);

        let (user_source, user_destination) = if is_a_to_b {
            (user_ata_a, user_ata_b)
        } else {
            (user_ata_b, user_ata_a)
        };

        let (vault_source, vault_destination) = if is_a_to_b {
            (self.pool.vault_a, self.pool.vault_b)
        } else {
            (self.pool.vault_b, self.pool.vault_a)
        };

        let accounts = vec![
            AccountMeta::new_readonly(self.pool.pk, false),
            AccountMeta::new_readonly(self.pool.swap_authority, false),
            AccountMeta::new(self.user, true), // signer
            AccountMeta::new(user_source, false),
            AccountMeta::new(vault_source, false),
            AccountMeta::new(vault_destination, false),
            AccountMeta::new(user_destination, false),
            AccountMeta::new(self.pool.pool_mint, false),
            AccountMeta::new(self.pool.pool_fee, false),
            AccountMeta::new_readonly(self.pool.token_program, false),
        ];

        Ok(Instruction {
            program_id: self.get_program_id(),
            accounts,
            data,
        })
    }
}
