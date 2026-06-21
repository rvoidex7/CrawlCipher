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

pub fn hash_to_seed(hash: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(hash.as_bytes());
    let result = hasher.finalize();
    // Use first 8 bytes as i64
    let bytes: [u8; 8] = result[0..8].try_into().unwrap_or([0; 8]);
    i64::from_le_bytes(bytes)
}
