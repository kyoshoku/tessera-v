use crate::constants::{ALPHAQ_PROGRAM_ID, ALPHAQ_SWAP_SELECTOR};
use crate::fetch::alphaq::AlphaqPool;
use crate::swap::SwapBuilder;
use crate::utils::get_ata;
use anyhow::Result;
use solana_program::pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub a_to_b: u8,          // Protocol-specific side indicator
    pub amount_in: u64,      // Input amount
    pub min_amount_out: u64, // Minimum output amount (slippage protection)
}

/// Alphaq swap instruction builder
pub struct AlphaqSwapBuilder {
    pool: AlphaqPool,
    user: Pubkey,
}

impl AlphaqSwapBuilder {
    pub fn new(pool: AlphaqPool, user: Pubkey) -> Self {
        Self { pool, user }
    }
}

impl SwapBuilder for AlphaqSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        ALPHAQ_PROGRAM_ID
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

impl AlphaqSwapBuilder {
    fn get_market_state(&self) -> Pubkey {
        // let pool = self.pool.pk;

        pubkey!("HZyb7Gv2pWTRYq8XuaeWBePQ8CDNhxigkNohZU2dLPEC")
    }

    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let a_to_b = input_mint.eq(&self.pool.mint_a) as u8;
        let swap_params = SwapParams {
            a_to_b,
            amount_in,
            min_amount_out,
        };

        // Alphaq swap data
        let mut data = Vec::with_capacity(18);
        data.extend_from_slice(&ALPHAQ_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(&self.user, &self.pool.mint_a, &self.pool.token_program_a);
        let user_ata_b = get_ata(&self.user, &self.pool.mint_b, &self.pool.token_program_b);

        let accounts = vec![
            AccountMeta::new(self.user, true), // signer
            AccountMeta::new_readonly(self.pool.pk, false),
            AccountMeta::new(self.get_market_state(), false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new(self.pool.vault_a, false),
            AccountMeta::new(self.pool.vault_b, false),
            AccountMeta::new(self.pool.token_a_authority, false),
            AccountMeta::new(self.pool.token_b_authority, false),
            AccountMeta::new(self.pool.vendor_authority, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
        ];

        Ok(Instruction {
            program_id: self.get_program_id(),
            accounts,
            data,
        })
    }
}
