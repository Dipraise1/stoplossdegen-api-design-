#!/bin/bash

echo "===== Solana Limit Order API & Testing Script ====="
echo ""

echo "Step 1: Running stop loss tests..."
echo "This will demonstrate how stop loss orders work in a simulated environment."
echo "---------------------------------------------------"
./test_stop_loss.sh
echo ""

echo "Step 2: Starting the API server..."
echo "This will start the Solana Limit Order API server."
echo "Use Ctrl+C to stop the server when you're done."
echo "---------------------------------------------------"
./run_api.sh 