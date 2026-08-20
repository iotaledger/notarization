#!/bin/bash

set -e

# Script to run all Proof of Inclusion examples.
# Usage: ./examples/poi/run.sh

echo "Running all Proof of Inclusion examples..."
echo "================================"

cargo run --release -p poi-examples --example 01_create_and_verify_transaction_proof
cargo run --release -p poi-examples --example 02_create_and_verify_multi_target_proof
cargo run --release -p poi-examples --example 03_reuse_verifier_for_multiple_proofs
cargo run --release -p poi-examples --example 04_create_and_verify_object_proof
cargo run --release -p poi-examples --example 05_create_and_verify_event_proof
cargo run --release -p poi-examples --example advanced_01_file_based_committee_cache

echo ""
echo "All Proof of Inclusion examples completed successfully!"
