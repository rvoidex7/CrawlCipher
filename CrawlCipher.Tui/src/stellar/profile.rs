use std::collections::HashMap;
use reqwest;
use serde::Deserialize;
use base64::{Engine as _, engine::general_purpose};
use stellar_strkey::ed25519::PrivateKey;

#[derive(Deserialize, Debug)]
pub struct AccountResponse {
    pub data: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct ProfileStats {
    pub total_kills: i64,
    pub max_length: i64,
    pub matches_played: i64,
    pub rank_points: i64,
}

pub async fn fetch_profile(account_id: &str) -> Result<ProfileStats, Box<dyn std::error::Error>> {
    let url = format!("https://horizon-testnet.stellar.org/accounts/{}", account_id);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?.json::<AccountResponse>().await?;

    Ok(ProfileStats {
        total_kills: parse_data_entry(&response.data, "total_kills"),
        max_length: parse_data_entry(&response.data, "max_length"),
        matches_played: parse_data_entry(&response.data, "matches_played"),
        rank_points: parse_data_entry(&response.data, "rank_points"),
    })
}

fn parse_data_entry(data: &HashMap<String, String>, key: &str) -> i64 {
    data.get(key)
        .and_then(|v| general_purpose::STANDARD.decode(v).ok())
        .and_then(|bytes| {
            if bytes.len() == 8 {
                let arr: [u8; 8] = bytes.try_into().unwrap_or([0; 8]);
                Some(i64::from_le_bytes(arr))
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// Simulates updating profile on chain (since full XDR building is complex for MVP without dedicated crate)
// In a real implementation, this would build a 'Manage Data' operation XDR, sign it with 'secret_key',
// and submit to Horizon.
// For now, we validate the key format and print the intended action.
pub async fn update_profile(
    secret_key: &str,
    stats: &ProfileStats
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate Secret Key
    let _key = PrivateKey::from_string(secret_key)
        .map_err(|_| "Invalid Stellar Secret Key (S...)")?;

    // 2. (Mock) Submit Transaction
    println!(">>> SUBMITTING TRANSACTION TO STELLAR TESTNET <<<");
    // Removed security vulnerability: Signer secret key should never be printed.
    println!("Operation: Manage Data");
    println!("  total_kills: {}", stats.total_kills);
    println!("  matches_played: {}", stats.matches_played);

    // Simulate network delay
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    println!(">>> TRANSACTION SUCCESS (Mock) <<<");
    Ok(())
}
