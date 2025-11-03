use anyhow::{Ok, Result};
use solana_pubkey::Pubkey;
use solana_sdk::instruction::AccountMeta;

use crate::{
    constants::{AQUIFER_POOL_AUTHORITY, AQUIFER_POOL_STATE, AQUIFER_PROGRAM_ID},
    hera::traits::{AccountMap, Pair, QuoteParams, Swap, SwapAndAccountMetas, SwapParams},
};

#[derive(Debug, Clone)]
pub struct AquiferPair {
    pub pk: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub vault_ata_a: Pubkey,
    pub vault_ata_b: Pubkey,
    pub oracle_a: Pubkey,
    pub oracle_b: Pubkey,
    pub pool_state: Pubkey,
    pub pool_authority: Pubkey,
}

impl Pair for AquiferPair {
    fn label(&self) -> String {
        format!("Aquifer: {} -> {}", self.base_mint, self.quote_mint)
    }

    fn program_id(&self) -> Pubkey {
        AQUIFER_PROGRAM_ID
    }

    fn key(&self) -> Pubkey {
        self.pk
    }

    fn get_base_mint(&self) -> Pubkey {
        self.base_mint
    }

    fn get_quote_mint(&self) -> Pubkey {
        self.quote_mint
    }

    fn get_mints(&self) -> Vec<Pubkey> {
        vec![self.base_mint, self.quote_mint]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![
            self.vault_a,
            self.vault_b,
            self.vault_ata_a,
            self.vault_ata_b,
            self.oracle_a,
            self.oracle_b,
            AQUIFER_POOL_AUTHORITY,
            AQUIFER_POOL_STATE,
        ]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<u64> {
        Ok(0)
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let account_metas = vec![
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::ID, false),
            AccountMeta::new(swap_params.input_token_account, true), // signer
            AccountMeta::new_readonly(self.token_program_b, false),
            AccountMeta::new(swap_params.output_token_account, false),
            AccountMeta::new_readonly(self.quote_mint, false),
            AccountMeta::new_readonly(self.token_program_a, false),
            AccountMeta::new(swap_params.input_token_account, false),
            AccountMeta::new_readonly(self.base_mint, false),
            AccountMeta::new_readonly(AQUIFER_POOL_AUTHORITY, false),
            AccountMeta::new(AQUIFER_POOL_STATE, false),
            AccountMeta::new_readonly(self.oracle_b, false),
            AccountMeta::new_readonly(self.oracle_a, false),
            AccountMeta::new(self.vault_b, false),
            AccountMeta::new(self.vault_ata_b, false),
            AccountMeta::new(self.vault_a, false),
            AccountMeta::new(self.vault_ata_a, false),
        ];

        Ok(SwapAndAccountMetas {
            swap: Swap::AQUIFER,
            account_metas,
        })
    }
}
