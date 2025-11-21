use anyhow::Result;
use base64;
use bs58;
use litesvm::LiteSVM;
use litesvm_token::CreateAssociatedTokenAccount;
use serde_json;
use solana_account::Account;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_program::last_restart_slot::LastRestartSlot;
use solana_pubkey::Pubkey;
use solana_sdk::{
    clock::{Clock, Epoch},
    epoch_schedule::EpochSchedule,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey,
    rent::Rent,
    slot_hashes::SlotHashes,
    slot_history::SlotHistory,
    sysvar::{
        fees::Fees, instructions::Instructions, recent_blockhashes::RecentBlockhashes, SysvarId,
    },
};
use solana_signer::Signer;
use spl_token::state::Account as TokenAccount;
use std::{
    fs::File,
    io::{Read, Write},
    str::FromStr,
};

use crate::constants::{
    ALPHAQ_PROGRAM_ID, ALPHAQ_PROGRAM_PATH, EXECUTOR_PROGRAM_ID, EXECUTOR_PROGRAM_PATH,
    OBRIC_PROGRAM_ID, OBRIC_PROGRAM_PATH, TESSERA_PROGRAM_ID, TESSERA_PROGRAM_PATH,
    ZEROFI_PROGRAM_ID, ZEROFI_PROGRAM_PATH,
};
use crate::utils::get_ata;

pub fn make_ata_account(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    payer: &Keypair,
    program_id: &Pubkey,
    amount: u64,
) -> Pubkey {
    let mut ata = get_ata(&payer.pubkey(), mint, program_id);
    if svm.get_account(&ata).is_none() {
        ata = CreateAssociatedTokenAccount::new(svm, payer, mint)
            .owner(&payer.pubkey())
            .token_program_id(program_id)
            .send()
            .unwrap();
    }

    if amount > 0 {
        let mut ata_account = svm.get_account(&ata).unwrap();
        let mut ata_token_account = TokenAccount::unpack(&ata_account.data).unwrap();
        ata_token_account.amount = amount;

        TokenAccount::pack(ata_token_account, &mut ata_account.data).unwrap();
        svm.set_account(ata, ata_account).unwrap();
    }

    ata
}

pub fn init_svm(dump_file_path: &str) -> Result<LiteSVM> {
    tracing_subscriber::fmt::init();
    std::env::set_var("RUST_LOG", "trace");

    let mut svm = LiteSVM::new()
        .with_sigverify(false)
        .with_lamports(1_000_000u64.wrapping_mul(LAMPORTS_PER_SOL));

    // svm.add_program_from_file(ATA_PROGRAM_ID, "data/ata.so")?;
    // svm.add_program_from_file(SPL_TOKEN_PROGRAM_ID, "data/spl_token.so")?;
    // svm.add_program_from_file(TOKEN2022_PROGRAM_ID, "data/token2022.so")?;

    svm.add_program_from_file(EXECUTOR_PROGRAM_ID, EXECUTOR_PROGRAM_PATH)?;
    svm.add_program_from_file(TESSERA_PROGRAM_ID, TESSERA_PROGRAM_PATH)?;
    svm.add_program_from_file(ZEROFI_PROGRAM_ID, ZEROFI_PROGRAM_PATH)?;
    svm.add_program_from_file(OBRIC_PROGRAM_ID, OBRIC_PROGRAM_PATH)?;
    svm.add_program_from_file(ALPHAQ_PROGRAM_ID, ALPHAQ_PROGRAM_PATH)?;

    // Load accounts from dump file and get slot number
    let (slot, timestamp) = load_accounts_from_dump(&mut svm, dump_file_path)?;

    // Set slot number from dump file
    let mut clock = svm.get_sysvar::<Clock>();
    clock.slot = slot;
    clock.unix_timestamp = timestamp as i64;
    svm.set_sysvar(&clock);

    Ok(svm)
}

fn load_accounts_from_dump(svm: &mut LiteSVM, dump_file_path: &str) -> Result<(u64, u64)> {
    // Read the dump file
    let mut file = File::open(dump_file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse JSON
    let dump_data: serde_json::Value = serde_json::from_str(&contents)?;

    // Extract slot number from dump file (if available)
    let slot = dump_data["slot"].as_u64().unwrap_or(0); // Default slot if not found
    let timestamp = dump_data["timestamp"].as_u64().unwrap_or(0);

    // Extract accounts array
    let accounts = dump_data["accounts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid dump file format: missing 'accounts' array"))?;

    // Load each account into the SVM
    for (i, account_data) in accounts.iter().enumerate() {
        let pubkey_str = account_data["pubkey"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pubkey' field in account {}", i))?;

        let pubkey = Pubkey::from_str(pubkey_str)?;

        let lamports = account_data["lamports"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing 'lamports' field in account {}", i))?;

        let data_bytes = if let Some(data_str) = account_data["data"].as_str() {
            // Try base58 first (new format), then base64 (legacy format)
            match bs58::decode(data_str).into_vec() {
                Ok(decoded) => decoded,
                Err(_) => {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_str)
                        .map_err(|e| {
                            anyhow::anyhow!(
                        "Failed to decode data string (base58 or base64) for account {}: {}",
                        i,
                        e
                    )
                        })?
                }
            }
        } else if let Some(data_array) = account_data["data"].as_array() {
            // Handle data as array of numbers (legacy format)
            data_array
                .iter()
                .map(|v| v.as_u64().unwrap_or(0) as u8)
                .collect::<Vec<u8>>()
        } else {
            return Err(anyhow::anyhow!(
                "Missing or invalid 'data' field in account {}",
                i
            ));
        };

        let owner_str = account_data["owner"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'owner' field in account {}", i))?;

        let owner = Pubkey::from_str(owner_str)?;

        let executable = account_data["executable"].as_bool().unwrap_or(false);

        let rent_epoch = account_data["rent_epoch"].as_u64().unwrap_or(0);

        // Create account
        let account = Account {
            lamports,
            data: data_bytes,
            owner,
            executable,
            rent_epoch,
        };

        // Add account to SVM
        svm.set_account(pubkey, account)
            .map_err(|e| anyhow::anyhow!("Failed to set account {}: {}", pubkey, e))?;
    }

    Ok((slot, timestamp))
}

pub fn dump_pool_accounts(addresses: Vec<Pubkey>, rpc_url: &str, output_file: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let mut addresses = addresses;
    addresses.push(Clock::id());
    addresses.push(RecentBlockhashes::id());
    addresses.push(LastRestartSlot::id());
    // addresses.push(EpochSchedule::id());
    // addresses.push(Fees::id());
    // addresses.push(Instructions::id());
    // addresses.push(Rent::id());
    // addresses.push(SlotHashes::id());
    // addresses.push(SlotHistory::id());

    let accounts = client.get_multiple_accounts(&addresses)?;

    // Convert accounts to JSON-serializable format
    let mut account_data = Vec::new();

    for (account, pubkey) in accounts.iter().zip(addresses.iter()) {
        if account.is_none() {
            continue;
        }

        let owner = account.as_ref().unwrap().owner.to_string();
        if owner.eq("BPFLoaderUpgradeab1e11111111111111111111111")
            || owner.eq("BPFLoader2111111111111111111111111111111111")
            || owner.eq("BPFLoader1111111111111111111111111111111111")
            || owner.eq("NativeLoader1111111111111111111111111111111")
        {
            continue;
        }

        println!("Account: {:?}", pubkey);

        let account_info = account.as_ref().unwrap();
        let account_json = serde_json::json!({
            "lamports": account_info.lamports,
            "pubkey": pubkey.to_string(),
            "data": bs58::encode(&account_info.data).into_string(),
            "owner": account_info.owner.to_string(),
            "executable": account_info.executable,
            "rent_epoch": account_info.rent_epoch,
        });
        account_data.push(account_json);
    }

    // Get current slot
    let slot = client.get_slot()?;
    let block_timestamp = client.get_block_time(slot)?;
    println!("slot: {}, block_timestamp: {}", slot, block_timestamp);

    // Create a serializable structure for dumping
    let dump_data = serde_json::json!({
        "slot": slot,
        "total_accounts": account_data.len(),
        "timestamp": block_timestamp,
        "accounts": account_data,
    });

    // Write to file
    let mut file = File::create(output_file)?;
    let json_string = serde_json::to_string_pretty(&dump_data)?;
    file.write_all(json_string.as_bytes())?;

    println!(
        "✅ Dumped {} accounts to: {}",
        account_data.len(),
        output_file
    );

    Ok(())
}

pub fn token_balance(svm: &LiteSVM, pubkey: &Pubkey) -> u64 {
    let account = svm.get_account(pubkey).unwrap_or_default();
    let state = TokenAccount::unpack(&account.data).ok().unwrap_or_default();
    state.amount
}
