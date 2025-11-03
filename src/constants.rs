use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;

/// Default RPC URL for mainnet
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Common pubkeys
pub const ATA_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const EXECUTOR_PROGRAM_ID: Pubkey = pubkey!("4oZM6h9uREbnjcWfy97EYPmDwUZijhjCnej6jZaHCGv1");
pub const EXECUTOR_PROGRAM_PATH: &str = "data/executor.so";

/// Tessera values
pub const TESSERA_SWAP_SELECTOR: [u8; 1] = [0x10];
pub const TESSERA_PROGRAM_ID: Pubkey = pubkey!("TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH");
pub const TESSERA_AUTHORITY: Pubkey = pubkey!("8ekCy2jHHUbW2yeNGFWYJT9Hm9FW7SvZcZK66dSZCDiF");
pub const TESSERA_PROGRAM_PATH: &str = "data/tesserav.so";

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

/// Humidifi values
pub const HUMIDIFI_SWAP_SELECTOR: [u8; 1] = [0x4];
pub const HUMIDIFI_PROGRAM_ID: Pubkey = pubkey!("9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp");

/// Aquifer values
pub const AQUIFER_SWAP_SELECTOR: [u8; 1] = [0x1];
pub const AQUIFER_PROGRAM_ID: Pubkey = pubkey!("AQU1FRd7papthgdrwPTTq5JacJh8YtwEXaBfKU3bTz45");
pub const AQUIFER_POOL_STATE: Pubkey = pubkey!("CNC5TaeNQEoSPfQKZ7GgfM4R8WYAJRKRSHFCHkf2H7ko");
pub const AQUIFER_POOL_AUTHORITY: Pubkey = pubkey!("5AVyF6qJBi8GxVjh6nh4Ew1DiJZugPxz9m58a8v2osk2");

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
