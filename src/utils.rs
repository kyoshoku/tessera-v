use anyhow::Result;
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_program::system_instruction::transfer;
use solana_pubkey as alt_pubkey;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::instruction::{close_account, sync_native};
use std::str::FromStr;

use crate::adapter::aquifer::AquiferAdapter;
use crate::adapter::{
    alphaq::AlphaqAdapter, goonfi::GoonfiAdapter, humidifi::HumidifiAdapter, obric::ObricAdapter,
    saros_amm::SarosAdapter, tessera::TesseraAdapter, zerofi::ZeroFiAdapter, DexAdapter,
};
use crate::config::Config;
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

pub fn get_adapter(protocol: &str, config: &Config) -> Result<Box<dyn DexAdapter>> {
    let client = RpcClient::new(config.get_rpc_url());

    let adapter: Box<dyn DexAdapter> = match protocol {
        "alphaq" => Box::new(AlphaqAdapter::new(client)),
        "tessera" => Box::new(TesseraAdapter::new(client)),
        "goonfi" => Box::new(GoonfiAdapter::new(client)),
        "obric" => Box::new(ObricAdapter::new(client)),
        "saros" => Box::new(SarosAdapter::new(client)),
        "humidifi" => Box::new(HumidifiAdapter::new(client)),
        "aquifer" => Box::new(AquiferAdapter::new(client)),
        "zerofi" => Box::new(ZeroFiAdapter::new(client)),
        _ => {
            anyhow::bail!("Unsupported protocol: {}", protocol);
        }
    };

    Ok(adapter)
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

#[derive(Clone)]
pub struct MiniAccount {
    pub data: Vec<u8>,
    pub owner: Pubkey,
}

pub fn make_rpc_getter<'a>(
    client: &'a RpcClient,
) -> impl FnMut(&Pubkey) -> Result<MiniAccount> + 'a {
    move |pk: &Pubkey| {
        let acc = client.get_account(pk)?;
        Ok(MiniAccount {
            data: acc.data,
            owner: acc.owner,
        })
    }
}

pub fn make_svm_getter<'a>(svm: &'a LiteSVM) -> impl FnMut(&Pubkey) -> Result<MiniAccount> + 'a {
    move |pk: &Pubkey| {
        let alt_pk = alt_pubkey::Pubkey::new_from_array(pk.to_bytes());
        let acc = svm
            .get_account(&alt_pk)
            .ok_or_else(|| anyhow::anyhow!("Account not found in SVM: {}", pk))?;
        Ok(MiniAccount {
            data: acc.data,
            owner: acc.owner,
        })
    }
}
