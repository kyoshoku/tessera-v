use crate::constants::OBRIC_PROGRAM_ID;
use crate::constants::OBRIC_SWAP_SELECTOR;
use crate::fetch::obric::ObricPool;
use crate::swap::SwapBuilder;
use crate::utils::get_ata;
use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

/// Common swap parameters
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub x_to_y: bool,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

/// Goonfi swap instruction builder
pub struct ObricSwapBuilder {
    pool: ObricPool,
    user: Pubkey,
}

impl ObricSwapBuilder {
    pub fn new(pool: ObricPool, user: Pubkey) -> Self {
        Self { pool, user }
    }
}

impl SwapBuilder for ObricSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        OBRIC_PROGRAM_ID
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

impl ObricSwapBuilder {
    /// Build the core swap instruction
    fn build_swap_instruction(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<Instruction> {
        let x_to_y = input_mint.eq(&self.pool.mint_a) as bool;
        let swap_params = SwapParams {
            x_to_y,
            amount_in,
            min_amount_out,
        };
        println!("swap_params: {:?}", swap_params);

        // Tessera swap data
        let mut data = Vec::with_capacity(25);
        data.extend_from_slice(&OBRIC_SWAP_SELECTOR);
        data.extend_from_slice(&borsh::to_vec(&swap_params)?);

        let user_ata_a = get_ata(&self.user, &self.pool.mint_a, &self.pool.token_program_a);
        let user_ata_b = get_ata(&self.user, &self.pool.mint_b, &self.pool.token_program_b);

        let accounts = vec![
            AccountMeta::new(self.pool.pk, false),
            AccountMeta::new_readonly(self.pool.protocol_fee_y, false),
            AccountMeta::new_readonly(self.pool.mint_sslp_x, false),
            AccountMeta::new(self.pool.vault_a, false),
            AccountMeta::new(self.pool.vault_b, false),
            AccountMeta::new(user_ata_a, false),
            AccountMeta::new(user_ata_b, false),
            AccountMeta::new_readonly(self.pool.protocol_fee_x, false),
            AccountMeta::new_readonly(self.pool.price_feed_x, false),
            AccountMeta::new_readonly(self.pool.price_feed_y, false),
            AccountMeta::new(self.user, true), // signer
            AccountMeta::new_readonly(spl_token::ID, false),
        ];

        Ok(Instruction {
            program_id: self.get_program_id(),
            accounts,
            data,
        })
    }
}
