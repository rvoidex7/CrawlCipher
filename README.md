# CrawlCipher

A full-stack, terminal-based dApp demonstrating **deterministic state hashing**, **cryptographic fairness**, and **Soroban smart contract integration** on the Stellar blockchain.

The primary focus of this project is resolving trust and manipulation issues in decentralized interactive systems through cryptographic proofs.

---

## Deployed Smart Contract (Stellar Testnet)

| Field | Value |
|---|---|
| **Contract ID** | `CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR` |
| **Network** | Stellar Testnet |
| **Explorer** | [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CBW63QCKMVUFUYA23CWBGEXOCCPMYIFBFCGBI4FRAV3NE4DPX264QYHR) |

---

## Quick Start

> **The proprietary native engine is closed-source and not included in this repository.**
> To run CrawlCipher, download the pre-packaged release which includes the compiled engine binary.

**[Download from Releases](https://github.com/rvoidex7/CrawlCipher/releases)**

Extract the archive and run the executable directly — no build step required.

---

## Project Description

CrawlCipher is built around the concept of **Trustless Session Verification**. It ensures that sessions cannot be faked, pre-simulated, or manipulated by the client.

### 1. Entropy from Block Hash (Anti-Simulation)
To prevent pre-calculation of optimal paths using known RNG seeds, the system initializes its core using the **latest Stellar Ledger Hash**.
- Upon starting a session, the Rust client calls the Horizon API.
- The fetched block hash is parsed into an `i64` integer and passed to the proprietary Native Engine via FFI.
- The session becomes inherently unpredictable yet completely deterministic given that specific seed.

### 2. Session Lock Mechanism (Soroban Smart Contract)
To prevent double-spending of NFT-based in-session items, a **Soroban Smart Contract** is used.
- **Pre-Session:** The Rust TUI invokes `lock_session` on the Stellar Testnet, temporarily locking the equipped loadout.
- **Post-Session:** Once the session ends, `unlock_session` is called, releasing the assets and recording the final result hash.

### 3. Proof of Execution (Fraud Proofing)
- **Input Logging:** The Native Engine records every keystroke and its exact tick during the session.
- **Session Hash:** At the end of a session, the engine serializes `[Seed + Config + Input Log + Secret Salt]` and runs it through SHA-256.
- **Verification:** This hash is submitted to the blockchain. A validator node can re-run the Input Log against the deterministic engine to mathematically verify the session is legitimate — similar to Optimistic Rollups.

### 4. Public On-Chain Profile
User statistics (Lifetime Kills, Max Length, Sessions Played) are written directly to the Stellar ledger using **Account Data Entries**, providing a verifiable and permanent public record.

---

## Architecture

```
CrawlCipher/
├── CrawlCipher.Terminal/     # Rust TUI frontend (ratatui)
├── smart-contracts/
│   └── session-lock/         # Soroban smart contract (Rust/WASM)
│       └── src/lib.rs        # lock_session / unlock_session / get_locked_assets
├── core-binaries/            # Proprietary native engine (.so / .dll)
├── deploy_contract.sh        # Deploy script for Stellar Testnet
└── install_stellar.sh        # Stellar CLI installer
```

---

## Smart Contract: session-lock

**Source:** `smart-contracts/session-lock/src/lib.rs`

| Function | Description |
|---|---|
| `lock_session(player, assets)` | Locks a list of asset IDs for a player's active session |
| `unlock_session(player, game_hash)` | Unlocks the session and records the result hash |
| `get_locked_assets(player)` | Returns the currently locked assets for a player |

All functions require the player's signature via `player.require_auth()`.

### Interact with the Deployed Contract

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
  --game_hash "sha256-hash-of-session"
```

---

## Building from Source

> For developers who want to compile the Rust frontend themselves.
> The proprietary engine binary must still be obtained from the [Releases](https://github.com/rvoidex7/CrawlCipher/releases) page and placed in `core-binaries/` before building.
> Full instructions: [docs/BUILD_INSTRUCTIONS.md](docs/BUILD_INSTRUCTIONS.md)

---

## License

- The Rust Terminal (Frontend) and Soroban Smart Contracts are licensed under the **Apache License 2.0**.
- The Pre-compiled Native Engine binaries (`core-binaries/`) are **Proprietary and All Rights Reserved**.

See the `LICENSE` file for details.
