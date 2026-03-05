# CrawlCipher

A full-stack, terminal-based dApp prototype demonstrating **deterministic game state hashing**, **cryptographic fairness**, and **Soroban smart contract integration** on the Stellar blockchain.

While this repository manifests as a tactical, inventory-based tactical game, its primary focus is on resolving trust and cheating issues in decentralized gaming through cryptographic proofs.

---

## 🔒 Cryptography & Blockchain Architecture

This project is built around the concept of **"Trustless Gameplay Verification."** It ensures that game sessions cannot be faked, pre-simulated, or manipulated by the client.

### 1. Entropy from Block Hash (Anti-Simulation)
To prevent players from pre-calculating optimal paths using known RNG seeds (pre-simulation exploit), the game initializes its core using the **latest Stellar Ledger Hash**.
* Upon starting a match, the Rust client calls the Horizon API.
* The fetched block hash is parsed into an `i64` integer and passed to the proprietary Native Engine via FFI.
* The game becomes inherently unpredictable yet completely deterministic given that specific seed.

### 2. Session Lock Mechanism (Smart Contract)
To prevent the "Double Spending" of NFT-based in-game items (e.g., using a weapon in a match while simultaneously selling it on a DEX), we utilize a **Soroban Smart Contract**.
* **Pre-Match:** The Rust TUI invokes the `lock_session` function on the Stellar Testnet, temporarily escrowing/locking the equipped loadout.
* **Post-Match:** Once the game ends, the `unlock_session` function is called, releasing the assets and recording the final stats.

### 3. Proof of Gameplay (Fraud Proofing)
How do we know the player didn't just hack their RAM to submit a high score?
* **Input Logging:** The Native Engine records every single keystroke and its exact tick (timestamp) during the match.
* **Game Hash:** At the "Game Over" screen, the engine serializes the `[Seed + Config + Input Log + Secret Salt]` and runs it through a `SHA-256` hashing algorithm.
* **Verification:** This irreversible hash is submitted to the blockchain. If a dispute arises, a validator node can re-run the exact Input Log against the deterministic engine. If the resulting hash matches the submitted hash, the gameplay is mathematically proven to be legitimate (similar to Optimistic Rollups).

### 4. Public On-Chain Profile
Player statistics (Lifetime Kills, Max Length, Matches Played) are written directly to the Stellar ledger using **Account Data Entries**, providing a verifiable and permanent public profile.

---

## 🕹️ The Game (Client)

The client serves as a visual wrapper for the underlying cryptographic engine.
* **Rust Terminal UI:** A lightweight, highly optimized TUI built with `ratatui`.
* **Proprietary Native Engine:** A compiled native library artifact (`.so` / `.dll`) that acts as the completely isolated, deterministic physics and logic brain.
* **Mechanics:** Manual movement with idle energy regeneration, A* pathfinding strikes, and a fully functional inventory system.

---

## 🚀 Quick Start (Demo Mode)

### Prerequisites
* Rust & Cargo installed.

### Run the App
If you simply want to test the terminal interface and the core loop:
```bash
# Clone the repository
git clone <your-repo-url>
cd <repo-name>

# Navigate to the frontend directory
cd CrawlCipher.Terminal
cargo run --release
```
*(Note: If you do not provide a Stellar Secret Key in the game's menu, it will default to a local "Ghost Protocol" offline mode).*

### 🛠 For Developers & Compiling
The core processing engine is **Proprietary (Closed Source)**. The frontend Rust TUI and Soroban smart contracts are Open Source (Apache 2.0).
If you want to deploy your own smart contract or build the project from scratch, please read the [Build & Compilation Instructions](docs/BUILD_INSTRUCTIONS.md). For simulation controls, select **Terminal Manual** from the main menu inside the application.

---

## ⚖️ License
* The Rust Terminal (Frontend) and Soroban Smart Contracts are licensed under the **Apache License 2.0**.
* The Pre-compiled Native Engine binaries (`core-binaries/`) are **Proprietary and All Rights Reserved**.
See the `LICENSE` file for details.
