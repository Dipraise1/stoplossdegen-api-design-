#!/bin/bash

# Load environment variables
source ./.env

# Build the application
echo "Building Solana Limit Order DEX..."
cargo build --release

# Run the application
echo "Starting Solana Limit Order DEX server..."
./target/release/solana_wallet_server 