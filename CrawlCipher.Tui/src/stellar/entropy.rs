use reqwest;
use serde::Deserialize;
use sha2::{Sha256, Digest};

#[derive(Deserialize)]
struct LedgerResponse {
    _embedded: Embedded,
}

#[derive(Deserialize)]
struct Embedded {
    records: Vec<Ledger>,
}

#[derive(Deserialize)]
struct Ledger {
    hash: String,
}

/// Fetches the latest block hash from the Stellar Horizon API to derive the simulation seed.
/// 
/// See: [Anti-Cheat-Verification.md](../../docs/r7/Development/Anti-Cheat-Verification.md#2-dynamic-entropy-via-stellar-ledger)
/// See: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
pub async fn fetch_latest_ledger_hash() -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://horizon-testnet.stellar.org/ledgers?order=desc&limit=1";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.json::<LedgerResponse>().await?;

    if let Some(record) = response._embedded.records.first() {
        Ok(record.hash.clone())
    } else {
        Err("No ledger records found".into())
    }
}

/// Fetches the hash of the ledger closed at a specific sequence number.
///
/// Used to derive the session seed from the ledger closed right after the on-chain
/// `lock_session` call (`sequence = lock_seq + 1`), so the seed is provably committed
/// before play begins rather than chosen after the fact.
///
/// See: [Anti-Cheat-Verification.md](../../docs/r7/Development/Anti-Cheat-Verification.md#2-dynamic-entropy-via-stellar-ledger)
/// See: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
pub async fn fetch_ledger_hash_by_sequence(sequence: u32) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://horizon-testnet.stellar.org/ledgers/{}", sequence);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        // Most commonly: the ledger hasn't closed yet (404). Caller polls briefly.
        return Err(format!("Ledger {} not available (status {})", sequence, response.status()).into());
    }

    let ledger = response.json::<Ledger>().await?;
    Ok(ledger.hash)
}

pub fn hash_to_seed(hash: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(hash.as_bytes());
    let result = hasher.finalize();
    // Use first 8 bytes as i64
    let bytes: [u8; 8] = result[0..8].try_into().unwrap_or([0; 8]);
    i64::from_le_bytes(bytes)
}
