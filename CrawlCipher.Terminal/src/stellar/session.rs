use tokio::process::Command;
use std::env;
use stellar_strkey::ed25519::{PrivateKey, PublicKey};
use ed25519_dalek::SigningKey;

fn get_public_address(secret_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pk = PrivateKey::from_string(secret_key)?;
    let signing_key = SigningKey::from_bytes(&pk.0);
    let verify_key = signing_key.verifying_key();
    let public_key = PublicKey(verify_key.to_bytes());
    Ok(public_key.to_string())
}

pub async fn lock_session(secret_key: &str, assets: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let contract_id = env::var("CRAWLCIPHER_CONTRACT_ID")
        .unwrap_or_else(|_| "CCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());

    if contract_id == "CCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" {
        println!(">>> WARNING: Using Mock Contract ID. To deploy to Testnet, run './deploy_contract.sh' first! <<<");
    }

    let assets_json = serde_json::to_string(&assets)?;
    let public_address = get_public_address(secret_key).unwrap_or_else(|_| "G_INVALID_KEY".to_string());

    println!(">>> CALLING SMART CONTRACT (TESTNET): lock_session <<<");
    println!("  Contract: {}", contract_id);
    println!("  Player:   {}", public_address);
    println!("  Locked Assets: {:?}", assets);

    let output = Command::new("stellar")
        .arg("contract")
        .arg("invoke")
        .arg("--id")
        .arg(&contract_id)
        .arg("--source-account")
        .arg(secret_key)
        .arg("--network")
        .arg("testnet")
        .arg("--")
        .arg("lock_session")
        .arg("--player")
        .arg(&public_address)
        .arg("--assets")
        .arg(&assets_json)
        .output()
        .await;

    match output {
        Ok(cmd_output) => {
            if cmd_output.status.success() {
                println!(">>> SMART CONTRACT SUCCESS: ASSETS LOCKED <<<");
                Ok(())
            } else {
                let err_msg = String::from_utf8_lossy(&cmd_output.stderr);
                if err_msg.contains("stellar: command not found") {
                     println!(">>> DEMO MODE: stellar-cli not found, simulating network success... <<<");
                     tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                     Ok(())
                } else {
                     Err(format!("Contract Invoke Failed: {}", err_msg).into())
                }
            }
        },
        Err(e) => {
             println!(">>> DEMO MODE: Error executing stellar-cli ({}), simulating network success... <<<", e);
             tokio::time::sleep(std::time::Duration::from_millis(800)).await;
             Ok(())
        }
    }
}

pub async fn unlock_session(secret_key: &str, game_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
    let contract_id = env::var("CRAWLCIPHER_CONTRACT_ID")
        .unwrap_or_else(|_| "CCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());

    let public_address = get_public_address(secret_key).unwrap_or_else(|_| "G_INVALID_KEY".to_string());

    println!(">>> CALLING SMART CONTRACT (TESTNET): unlock_session <<<");
    println!("  Player:           {}", public_address);
    println!("  Submitting Proof: {}", game_hash);

    let output = Command::new("stellar")
        .arg("contract")
        .arg("invoke")
        .arg("--id")
        .arg(&contract_id)
        .arg("--source-account")
        .arg(secret_key)
        .arg("--network")
        .arg("testnet")
        .arg("--")
        .arg("unlock_session")
        .arg("--player")
        .arg(&public_address)
        .arg("--_game_hash")
        .arg(game_hash)
        .output()
        .await;

    match output {
        Ok(cmd_output) => {
            if cmd_output.status.success() {
                println!(">>> SMART CONTRACT SUCCESS: ASSETS UNLOCKED <<<");
                Ok(())
            } else {
                let err_msg = String::from_utf8_lossy(&cmd_output.stderr);
                if err_msg.contains("stellar: command not found") {
                     println!(">>> DEMO MODE: stellar-cli not found, simulating network success... <<<");
                     tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                     Ok(())
                } else {
                     Err(format!("Contract Invoke Failed: {}", err_msg).into())
                }
            }
        },
        Err(e) => {
             println!(">>> DEMO MODE: Error executing stellar-cli ({}), simulating network success... <<<", e);
             tokio::time::sleep(std::time::Duration::from_millis(800)).await;
             Ok(())
        }
    }
}
