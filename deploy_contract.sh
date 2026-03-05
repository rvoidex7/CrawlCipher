#!/bin/bash
set -e

echo "Building Smart Contract..."
cd smart-contracts/session-lock
cargo build --target wasm32-unknown-unknown --release
cd ../..

echo ""
echo "Deploying to Stellar Testnet..."
echo "Please set your SECRET_KEY environment variable before running this script."

if [ -z "$SECRET_KEY" ]; then
    echo "Secret key is required for deployment."
    echo "Example: export SECRET_KEY=SXXXXXXXXXXXXXXXX"
else
    echo "Deploying... (This may take a few seconds)"

    CONTRACT_ID=$(STELLAR_ACCOUNT="$SECRET_KEY" stellar contract deploy \
      --wasm smart-contracts/session-lock/target/wasm32-unknown-unknown/release/session_lock.wasm \
      --network testnet)

    echo ""
    echo "✅ Deployment Successful!"
    echo "Your Contract ID is: $CONTRACT_ID"
    echo ""
    echo "Please copy this Contract ID."
    echo "You can set it as an environment variable before running the game:"
    echo "export CRAWLCIPHER_CONTRACT_ID=\"$CONTRACT_ID\""
fi
