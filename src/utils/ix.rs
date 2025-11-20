use crate::constants::{EXECUTOR_PROGRAM_ID, WSOL_MINT};
use crate::utils::get_anchor_discriminator;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction::transfer;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::instruction::{close_account, sync_native};

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
