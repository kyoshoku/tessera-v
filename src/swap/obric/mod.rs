use crate::config::Config;
use crate::constants::{OBRIC_SWAP_SELECTOR, WSOL_MINT};
use crate::fetch::obric::ObricPool;
use crate::swap::SwapBuilder;
use crate::utils::{build_unwrap_sol_instruction, build_wrap_sol_instruction, get_ata};
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
    pub x_to_y: bool,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

/// Goonfi swap instruction builder
pub struct ObricSwapBuilder {
    pool: ObricPool,
    config: Config,
    user: Pubkey,
}

impl ObricSwapBuilder {
    pub fn new(pool: ObricPool, user: Pubkey, config: Config) -> Self {
        Self { pool, user, config }
    }
}

impl SwapBuilder for ObricSwapBuilder {
    fn get_program_id(&self) -> Pubkey {
        self.config.get_obric_program_id()
    }

    /// Build swap instruction with automatic side detection based on input token
    fn build_swap(
        &self,
        input_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        _wrap_sol: bool,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];

        let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();

        let wsol_ata =
            get_associated_token_address_with_program_id(&self.user, &wsol_mint, &spl_token::ID);
        let wrap_sol = self.pool.mint_a.eq(&wsol_mint) || self.pool.mint_b.eq(&wsol_mint);

        if wrap_sol {
            let wrap_sol_amount = if input_mint.eq(&self.pool.mint_a) {
                amount_in
            } else {
                0
            };
            instructions.extend(build_wrap_sol_instruction(
                &self.user,
                &wsol_ata,
                wrap_sol_amount,
            ));
        }

        // Build the swap instruction
        instructions.push(
            self.build_swap_instruction(input_mint, amount_in, min_amount_out)
                .unwrap(),
        );

        if wrap_sol {
            instructions.extend(build_unwrap_sol_instruction(&self.user, &wsol_ata));
        }

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
            program_id: self.config.get_obric_program_id(),
            accounts,
            data,
        })
    }
}
