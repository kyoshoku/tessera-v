use sha2::{Digest, Sha256};
use solana_program::system_instruction::transfer;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::instruction::{close_account, sync_native};
use std::str::FromStr;

use crate::constants::{ATA_PROGRAM_ID, EXECUTOR_PROGRAM_ID, WSOL_MINT};

/// Helper function to get Pubkey from string constant
pub fn get_pubkey_from_str(s: &str) -> Result<Pubkey, solana_sdk::pubkey::ParsePubkeyError> {
    Pubkey::from_str(s)
}

// Manual implementation of get_associated_token_address
pub fn get_ata(owner: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn get_anchor_discriminator(name: &str) -> Vec<u8> {
    let discriminator = &Sha256::digest(name.as_bytes())[0..8];
    discriminator.to_vec()
}

pub fn build_wrap_sol_instruction(user: &Pubkey, ata: &Pubkey, lamports: u64) -> Vec<Instruction> {
    let mut ixs = vec![];

    ixs.push(create_associated_token_account_idempotent(
        user,
        user,
        &WSOL_MINT,
        &spl_token::ID,
    ));
    if lamports > 0 {
        ixs.push(transfer(user, ata, lamports));
    }
    ixs.push(sync_native(&spl_token::ID, ata).unwrap());
    ixs
}

pub fn build_unwrap_sol_instruction(user: &Pubkey, ata: &Pubkey) -> Vec<Instruction> {
    let mut ixs = vec![];

    ixs.push(close_account(&spl_token::ID, ata, user, user, &[user]).unwrap());
    ixs
}

pub fn build_executor_instruction(signer: Pubkey, ix: Instruction) -> Instruction {
    // Extend executor ix data
    let mut data = get_anchor_discriminator("global:execute_swap");
    data.extend_from_slice(&ix.data);
    println!("Executor data len: {}", data.len());

    let mut accounts = vec![];
    accounts.push(AccountMeta {
        pubkey: signer,
        is_signer: true,
        is_writable: true,
    });
    accounts.push(AccountMeta {
        pubkey: ix.program_id,
        is_signer: false,
        is_writable: false,
    });
    accounts.push(AccountMeta {
        pubkey: solana_sdk::sysvar::instructions::ID,
        is_signer: false,
        is_writable: false,
    });

    accounts.extend(ix.accounts);

    Instruction {
        program_id: EXECUTOR_PROGRAM_ID,
        accounts,
        data,
    }
}
