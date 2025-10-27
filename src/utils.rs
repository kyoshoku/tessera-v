use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::constants::ATA_PROGRAM_ID;

/// Helper function to get Pubkey from string constant
pub fn get_pubkey_from_str(s: &str) -> Result<Pubkey, solana_sdk::pubkey::ParsePubkeyError> {
    Pubkey::from_str(s)
}

// Manual implementation of get_associated_token_address
pub fn get_ata(owner: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> Pubkey {
    let associated_token_program_id = Pubkey::from_str(ATA_PROGRAM_ID).unwrap();
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &associated_token_program_id,
    )
    .0
}
