use anyhow::Result;
use solana_program::program_pack::Pack;
use solana_pubkey::Pubkey;
use spl_token::state::Mint;

use crate::constants::{CBTC_MINT, JUP_MINT, USDC_MINT, USDT_MINT, WETH_MINT, WSOL_MINT};

pub fn get_token_mint(mint: &str) -> Result<Pubkey> {
    match mint {
        "USDC" => Ok(USDC_MINT),
        "USDT" => Ok(USDT_MINT),
        "SOL" => Ok(WSOL_MINT),
        "JUP" => Ok(JUP_MINT),
        "cbBTC" => Ok(CBTC_MINT),
        "WETH" => Ok(WETH_MINT),
        _ => anyhow::bail!("Invalid mint: {}", mint),
    }
}

pub fn get_token_decimals(data: &[u8]) -> Result<u8> {
    let mint_account = Mint::unpack_unchecked(&data[0..82])?;
    let decimals = mint_account.decimals;
    Ok(decimals)
}
