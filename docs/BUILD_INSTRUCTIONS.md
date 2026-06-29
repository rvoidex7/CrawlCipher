# Build & Compilation Instructions

This document is intended for users who wish to build the project from its source code.

**Note:** The deterministic native engine (`.dll` or `.so`) is developed locally in C# (`CrawlCipher.Core`) but is excluded from the public Git repository via `.gitignore` to maintain the integrity of the anti-cheat verification mechanisms. Therefore:
- **For Public Git Clones:** You must download the pre-compiled binaries from the GitHub Releases page and place them in the `core-binaries/` folder before building the TUI.
- **For Local Development (if source is present):** You can compile the entire project (Core Engine + TUI) from source.

## Prerequisites
* **Rust & Cargo** (for the Terminal UI and Smart Contracts)
* **Stellar CLI** (for Smart Contract deployment)

## 1. Rust Terminal UI (Open Source)
The frontend terminal interface is completely open-source (Apache 2.0).

### Linux
```bash
./build-linux.sh
```
This script will automatically compile the Rust project and package the executable along with the native engine binary (from `core-binaries/`) into the `output/` directory.

### Windows
Run the `build-windows.bat` file. It will compile the Rust `.exe` and package it with the native engine binary.

## 2. Soroban Smart Contracts (Open Source)
To compile the `session-lock` contract into a WebAssembly (`.wasm`) target:
```bash
cd smart-contracts/session-lock
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```
To deploy it to the Stellar Testnet, use the root level script:
```bash
export SECRET_KEY="S_YOUR_TESTNET_SECRET_KEY"
./deploy_contract.sh
```

## 3. Core Engine Compilation (Local Dev Only)
If you have access to the local `CrawlCipher.Core` C# source code, you can compile it into a native shared library using .NET 8.0 NativeAOT:
- **Automatic Dev Build:** Run `./dev-build-linux.sh` (Linux) or `dev-build-windows.bat` (Windows) at the project root. This restores, builds, and publishes both the C# Core Engine and Rust TUI into the `output/` directory.
- **Manual Publish:**
  ```bash
  cd CrawlCipher.Core
  dotnet publish -c Release -r linux-x64 -p:PublishAot=true
  # Copy CrawlCipher.Core.so to core-binaries/libCrawlCipher.Core.so
  ```
