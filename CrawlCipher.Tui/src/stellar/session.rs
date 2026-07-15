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
/// Invokes the `lock_session` function on the Soroban smart contract to lock the player's active loadout.
/// 
/// See: [Architecture.md](../../docs/r7/Development/Architecture.md)
/// See: https://rvoidex7.github.io/r7notes/Github-Projects/Architecture
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

    // 1. Evaluate Soroban Contract Invocation Output
    match output {
        Ok(cmd_output) => {
            if cmd_output.status.success() {
                println!(">>> SMART CONTRACT SUCCESS: ASSETS LOCKED <<<");
                Ok(())
            } else {
                let err_msg = String::from_utf8_lossy(&cmd_output.stderr);
                
                // 2. Offline Sandbox Fallback Rule
                // If the player does not have 'stellar-cli' installed, we do not crash the game.
                // We notify the player and fallback to a local simulation of network success
                // to allow offline testing of the gameplay loop.
                if err_msg.contains("stellar: command not found") {
                     println!(">>> DEMO MODE: stellar-cli not found, simulating network success... <<<");
                     tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                     Ok(())
                } else {
                     // The CLI is present, but the smart contract reported an actual validation/signature error.
                     Err(format!("Contract Invoke Failed: {}", err_msg).into())
                }
            }
        },
        Err(e) => {
             // 3. Spawning Error Fallback
             // If the shell fails to spawn the process, fallback to sandbox demo mode.
             println!(">>> DEMO MODE: Error executing stellar-cli ({}), simulating network success... <<<", e);
             tokio::time::sleep(std::time::Duration::from_millis(800)).await;
             Ok(())
        }
    }
}

/// Reads the lock-time ledger sequence for a player's active session lock via the
/// `get_lock_seq` contract getter.
///
/// Returns `Ok(None)` both when the contract has no active lock for the player and when
/// the chain is unreachable in the same "demo mode" sense as `lock_session`/`unlock_session`
/// (e.g. `stellar-cli` not installed) — callers should fall back to legacy entropy in that case.
///
/// See: [Anti-Cheat-Verification.md](../../docs/r7/Development/Anti-Cheat-Verification.md)
/// See: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
pub async fn get_lock_seq(secret_key: &str) -> Result<Option<u32>, Box<dyn std::error::Error>> {
    let contract_id = env::var("CRAWLCIPHER_CONTRACT_ID")
        .unwrap_or_else(|_| "CCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());

    let public_address = get_public_address(secret_key).unwrap_or_else(|_| "G_INVALID_KEY".to_string());

    println!(">>> CALLING SMART CONTRACT (TESTNET): get_lock_seq <<<");
    println!("  Player: {}", public_address);

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
        .arg("get_lock_seq")
        .arg("--player")
        .arg(&public_address)
        .output()
        .await;

    match output {
        Ok(cmd_output) => {
            if cmd_output.status.success() {
                let stdout = String::from_utf8_lossy(&cmd_output.stdout);
                let trimmed = stdout.trim().trim_matches('"');
                match trimmed.parse::<u32>() {
                    Ok(seq) => {
                        println!(">>> SMART CONTRACT SUCCESS: lock_seq = {} <<<", seq);
                        Ok(Some(seq))
                    }
                    // "null" (no active lock recorded) or unparseable output.
                    Err(_) => Ok(None),
                }
            } else {
                let err_msg = String::from_utf8_lossy(&cmd_output.stderr);

                // Same offline sandbox fallback rule as lock_session/unlock_session: if
                // stellar-cli isn't installed, don't crash — let the caller fall back.
                if err_msg.contains("stellar: command not found") {
                    println!(">>> DEMO MODE: stellar-cli not found, cannot read lock_seq <<<");
                    Ok(None)
                } else {
                    Err(format!("Contract Invoke Failed: {}", err_msg).into())
                }
            }
        }
        Err(e) => {
            println!(">>> DEMO MODE: Error executing stellar-cli ({}), cannot read lock_seq <<<", e);
            Ok(None)
        }
    }
}

/// Invokes the `unlock_session` function on the Soroban smart contract to submit the session proof hash and release locked assets.
/// 
/// See: [Anti-Cheat-Verification.md](../../docs/r7/Development/Anti-Cheat-Verification.md)
/// See: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
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
