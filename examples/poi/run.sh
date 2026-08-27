#!/bin/bash

set -e

# Script to run all Proof of Inclusion examples.
# Usage: ./examples/poi/run.sh

echo "Running all Proof of Inclusion examples..."
echo "================================"
echo "Using the active IOTA CLI environment and wallet."
echo "This run submits seven locked Notarization transactions."
echo "Genesis-anchored examples run in separate processes and may repeat the committee walk."
echo "On mainnet, all seven transactions consume paid gas from the active wallet."
echo ""

cargo run --release -p poi-examples --example 01_transaction_proof
cargo run --release -p poi-examples --example 02_multi_target_proof
cargo run --release -p poi-examples --example 03_reuse_verifier
cargo run --release -p poi-examples --example 04_object_proof
cargo run --release -p poi-examples --example 05_event_proof
cargo run --release -p poi-examples --example advanced_01_committee_cache

echo ""
echo "All Proof of Inclusion examples completed successfully!"
