# CrawlCipher

A full-stack, terminal-based dApp demonstrating **deterministic state hashing**, **cryptographic fairness**, and **Soroban smart contract integration** on the Stellar blockchain.

While this repository manifests as a tactical, inventory-based, its primary focus is resolving trust and cheating issues in decentralized gaming through cryptographic proofs.

---

## Deployed Smart Contract (Stellar Testnet)

| Field | Value |
|---|---|
| **Contract ID** | `CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR` |
| **Network** | Stellar Testnet |
| **Explorer** | [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR) |

---

## Project Description

CrawlCipher is built around the concept of **Trustless Verification**. It ensures that sessions cannot be faked, pre-simulated, or manipulated by the client.

### 1. Entropy from Block Hash (Anti-Simulation)
To prevent players from pre-calculating optimal paths using known RNG seeds, the initializes its core using the **latest Stellar Ledger Hash**.
- Upon starting a match, the Rust client calls the Horizon API.
- The fetched block hash is parsed into an `i64` integer and passed to the proprietary Native Engine via FFI.
- The becomes inherently unpredictable yet completely deterministic given that specific seed.

### 2. Session Lock Mechanism (Soroban Smart Contract)
To prevent double-spending of NFT-based items, a **Soroban Smart Contract** is used.
- **Pre-Match:** The Rust TUI invokes `lock_session` on the Stellar Testnet, temporarily locking the equipped loadout.
- **Post-Match:** Once the ends, `unlock_session` is called, releasing the assets and recording the final stats.

### 3. Proof (Fraud Proofing)
- **Input Logging:** The Native Engine records every keystroke and its exact tick during the match.
- **Hash:** At the "Game Over" screen, the engine serializes `[Seed + Config + Input Log + Secret Salt]` and runs it through SHA-256.
- **Verification:** This hash is submitted to the blockchain. A validator node can re-run the Input Log against the deterministic engine to mathematically verify.

### 4. Public On-Chain Profile
Player statistics (Lifetime Kills, Max Length, Matches Played) are written directly to the Stellar ledger using **Account Data Entries**, providing a verifiable and permanent public profile.

---

## Architecture

```
CrawlCipher/
├── CrawlCipher.Terminal/     # Rust TUI frontend (ratatui)
├── CrawlCipher.Core/         # .NET core library
├── smart-contracts/
│   └── session-lock/         # Soroban smart contract (Rust/WASM)
│       └── src/lib.rs        # lock_session / unlock_session / get_locked_assets
├── core-binaries/            # Proprietary native engine (.so / .dll)
├── deploy_contract.sh        # Deploy script for Stellar Testnet
└── install_stellar.sh        # Stellar CLI installer
```

---

## Setup & Installation

### Prerequisites

- Rust & Cargo ([install](https://rustup.rs))
- Stellar CLI ([install guide](https://developers.stellar.org/docs/tools/developer-tools/stellar-cli))
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

### 1. Clone the Repository

```bash
git clone https://github.com/rvoidex7/CrawlCipher.git
cd CrawlCipher
```

### 2. Run the Game (Demo Mode)

```bash
cd CrawlCipher.Terminal
cargo run --release
```

> If no Stellar Secret Key is provided in the menu, it defaults to local **Ghost Protocol** offline mode.

### 3. Deploy the Smart Contract (Optional)

Set your secret key and run the deploy script:

```bash
export SECRET_KEY=YOUR_STELLAR_SECRET_KEY
bash deploy_contract.sh
```

Or deploy manually:

```bash
# Build the WASM contract
cd smart-contracts/session-lock
cargo build --target wasm32-unknown-unknown --release

# Deploy to testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/session_lock.wasm \
  --source-account YOUR_SECRET_KEY \
  --network testnet
```

### 4. Interact with the Deployed Contract

```bash
# Lock a session
stellar contract invoke \
  --id CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR \
  --source YOUR_SECRET_KEY \
  --network testnet \
  -- lock_session \
  --player YOUR_PUBLIC_KEY \
  --assets '["sword_001","shield_002"]'

# Check locked assets
stellar contract invoke \
  --id CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR \
  --source YOUR_SECRET_KEY \
  --network testnet \
  -- get_locked_assets \
  --player YOUR_PUBLIC_KEY

# Unlock session
stellar contract invoke \
  --id CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR \
  --source YOUR_SECRET_KEY \
  --network testnet \
  -- unlock_session \
  --player YOUR_PUBLIC_KEY \
  --game_hash "sha256hashofgamesession"
```

---

## Smart Contract: session-lock

**Source:** `smart-contracts/session-lock/src/lib.rs`

| Function | Description |
|---|---|
| `lock_session(player, assets)` | Locks a list of asset IDs for a player's active session |
| `unlock_session(player, game_hash)` | Unlocks the session and records the game result hash |
| `get_locked_assets(player)` | Returns the currently locked assets for a player |

All functions require the player's signature via `player.require_auth()`.

---

## License

- The Rust Terminal (Frontend) and Soroban Smart Contracts are licensed under the **Apache License 2.0**.
- The Pre-compiled Native Engine binaries (`core-binaries/`) are **Proprietary and All Rights Reserved**.

See the `LICENSE` file for details.
