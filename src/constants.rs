use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;

/// Default RPC URL for mainnet
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Common pubkeys
pub const ATA_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const EXECUTOR_PROGRAM_ID: Pubkey = pubkey!("G4KYKzZdhuLfW4E1vZd2oDdZvpsjN98aNp2E45CC4tXK");

/// Tessera values
pub const TESSERA_SWAP_SELECTOR: [u8; 1] = [0x10];
pub const TESSERA_PROGRAM_ID: Pubkey = pubkey!("TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH");
pub const TESSERA_AUTHORITY: Pubkey = pubkey!("8ekCy2jHHUbW2yeNGFWYJT9Hm9FW7SvZcZK66dSZCDiF");

/// Goonfi values
pub const GOONFI_SWAP_SELECTOR: [u8; 1] = [0x2];
pub const GOONFI_PROGRAM_ID: Pubkey = pubkey!("goonERTdGsjnkZqWuVjs73BZ3Pb9qoCUdBUL17BnS5j");

/// Obric values
pub const OBRIC_SWAP_SELECTOR: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];
pub const OBRIC_PROGRAM_ID: Pubkey = pubkey!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");

/// Alphaq values
pub const ALPHAQ_SWAP_SELECTOR: [u8; 1] = [12];
pub const ALPHAQ_PROGRAM_ID: Pubkey = pubkey!("ALPHAQmeA7bjrVuccPsYPiCvsi428SNwte66Srvs4pHA");

/// Saros values
pub const SAROS_SWAP_SELECTOR: [u8; 1] = [0x1];
pub const SAROS_PROGRAM_ID: Pubkey = pubkey!("SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr");

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
