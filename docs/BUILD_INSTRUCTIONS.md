# Build & Compilation Instructions

This document is intended for users who wish to build the project from its source code.

**Note:** The deterministic native engine (`.dll` or `.so`) is proprietary and closed-source. This repository assumes you have downloaded the pre-compiled proprietary binaries into the `core-binaries/` folder before building the frontend.

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

## 3. Proprietary Engine
The internal deterministic engine logic is closed-source to protect the integrity of the anti-cheat verification mechanisms.
* Developers must place the official native engine binary (`.so` on Linux, `.dll` on Windows) inside the `core-binaries/` directory before building the TUI wrapper.
