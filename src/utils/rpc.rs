use anyhow::Result;
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_commitment_config::CommitmentConfig;
use solana_pubkey as alt_pubkey;
use solana_sdk::pubkey::Pubkey;

use std::str::FromStr;

use crate::adapter::{
    alphaq::AlphaqAdapter, aquifer::AquiferAdapter, bisonfi::BisonfiAdapter, goonfi::GoonfiAdapter,
    humidifi::HumidifiAdapter, obric::ObricAdapter, saros_amm::SarosAdapter, tessera::TesseraAdapter,
    zerofi::ZeroFiAdapter, DexAdapter,
};
use crate::config::Config;
use crate::constants::ATA_PROGRAM_ID;

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
        "bisonfi" => Box::new(BisonfiAdapter::new(client)),
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

pub fn get_pma_with_filter(
    client: &RpcClient,
    program_id: &Pubkey,
    size: u64,
    memcmps: Vec<(usize, Pubkey)>,
) -> Result<Vec<(Pubkey, Account)>> {
    let mut filters = vec![RpcFilterType::DataSize(size)];
    for (offset, mint) in memcmps {
        filters.push(RpcFilterType::Memcmp(Memcmp::new(
            offset,
            MemcmpEncodedBytes::Base58(mint.to_string()),
        )));
    }

    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::finalized()),
            ..Default::default()
        },
        ..Default::default()
    };

    let accounts = client.get_program_accounts_with_config(program_id, config)?;
    Ok(accounts)
}
