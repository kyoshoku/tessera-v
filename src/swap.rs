use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use solana_sdk::instruction::Instruction;
use solana_sdk::{
    message::{Message, VersionedMessage},
    signature::Signer,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};

use crate::{
    adapter::{DexAdapter, PoolData},
    config::Config,
    constants::WSOL_MINT,
    utils::{build_unwrap_sol_instruction, build_wrap_sol_instruction, get_ata},
};

pub fn build_swap_transaction(
    adapter: Box<dyn DexAdapter>,
    pool_data: Box<dyn PoolData>,
    a_to_b: bool,
    amount_in: u64,
    min_amount_out: u64,
    config: &Config,
) -> Result<VersionedTransaction> {
    let input_token = if a_to_b {
        pool_data.get_mint_a()
    } else {
        pool_data.get_mint_b()
    };

    // Load payer from config
    let payer = config
        .get_payer()
        .map_err(|e| anyhow::anyhow!("Failed to get payer: {}", e))?;
    let user = payer.pubkey();
    println!("User: {}", user);

    let mut pre_ixs = vec![];

    let wsol_ata = get_associated_token_address_with_program_id(&user, &WSOL_MINT, &spl_token::ID);
    let wrap_sol = input_token.eq(&WSOL_MINT) || input_token.eq(&WSOL_MINT);
    if wrap_sol {
        let wrap_sol_amount = if input_token.eq(&WSOL_MINT) {
            amount_in
        } else {
            0
        };
        pre_ixs.extend(build_wrap_sol_instruction(
            &user,
            &wsol_ata,
            wrap_sol_amount,
        ));
    }

    // Build swap instructions
    let swap_ixs = adapter.build_swap_instructions(
        pool_data.as_ref(),
        &user,
        a_to_b,
        amount_in,
        min_amount_out,
    )?;

    // Determine output token
    let output_token = if input_token.eq(&pool_data.get_mint_a()) {
        pool_data.get_mint_b()
    } else {
        pool_data.get_mint_a()
    };

    let output_ata = get_ata(&user, &output_token, &spl_token::ID);
    if !output_ata.eq(&wsol_ata) {
        let client = RpcClient::new(config.get_rpc_url());
        let output_ata_exists = client.get_account(&output_ata).is_ok();
        if !output_ata_exists {
            pre_ixs.push(create_associated_token_account_idempotent(
                &user,
                &user,
                &output_token,
                &spl_token::ID,
            ));
        }
    }

    let post_ixs = if wrap_sol {
        build_unwrap_sol_instruction(&user, &wsol_ata)
    } else {
        vec![]
    };

    let ixs = [pre_ixs, swap_ixs, post_ixs].concat();

    // Get recent blockhash from RPC
    let client = RpcClient::new(config.get_rpc_url());
    let recent_blockhash = client.get_latest_blockhash()?;

    // Build the transaction
    let message = Message::new_with_blockhash(&ixs, Some(&user), &recent_blockhash);
    let versioned_message = VersionedMessage::Legacy(message);
    let transaction = VersionedTransaction::try_new(versioned_message, &[&payer])?;

    Ok(transaction)
}

pub fn build_simulate_ixs(
    adapter: &Box<dyn DexAdapter>,
    pool_data: Box<dyn PoolData>,
    user: &Pubkey,
    a_to_b: bool,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Vec<Instruction>> {
    let input_token = if a_to_b {
        pool_data.get_mint_a()
    } else {
        pool_data.get_mint_b()
    };

    let mut pre_ixs = vec![];

    let wsol_ata = get_associated_token_address_with_program_id(&user, &WSOL_MINT, &spl_token::ID);
    let wrap_sol = input_token.eq(&WSOL_MINT) || input_token.eq(&WSOL_MINT);
    if wrap_sol {
        let wrap_sol_amount = if input_token.eq(&WSOL_MINT) {
            amount_in
        } else {
            0
        };
        pre_ixs.extend(build_wrap_sol_instruction(
            &user,
            &wsol_ata,
            wrap_sol_amount,
        ));
    }

    // Build swap instructions
    let swap_ixs = adapter.build_swap_instructions(
        pool_data.as_ref(),
        &user,
        a_to_b,
        amount_in,
        min_amount_out,
    )?;

    let post_ixs = if wrap_sol {
        build_unwrap_sol_instruction(&user, &wsol_ata)
    } else {
        vec![]
    };

    let ixs = [pre_ixs, swap_ixs, post_ixs].concat();
    Ok(ixs)
}
