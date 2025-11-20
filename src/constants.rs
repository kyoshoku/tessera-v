use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;

/// Default RPC URL for mainnet
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Common pubkeys
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
pub const OBRIC_PROGRAM_PATH: &str = "data/obric.so";

/// Alphaq values
pub const ALPHAQ_SWAP_SELECTOR: [u8; 1] = [12];
pub const ALPHAQ_PROGRAM_ID: Pubkey = pubkey!("ALPHAQmeA7bjrVuccPsYPiCvsi428SNwte66Srvs4pHA");
pub const ALPHAQ_PROGRAM_PATH: &str = "data/alphaq.so";

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

/// ZeroFi values
pub const ZEROFI_SWAP_SELECTOR: [u8; 1] = [0x6];
pub const ZEROFI_PROGRAM_ID: Pubkey = pubkey!("ZERor4xhbUycZ6gb9ntrhqscUcZmAbQDjEAtCf4hbZY");
pub const ZEROFI_PROGRAM_PATH: &str = "data/zerofi.so";

pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const JUP_MINT: Pubkey = pubkey!("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN");
pub const CBTC_MINT: Pubkey = pubkey!("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij");
pub const WETH_MINT: Pubkey = pubkey!("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs");