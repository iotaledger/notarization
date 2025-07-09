#!/bin/bash

# Script to run all notarization examples
# Usage: ./run.sh
# Make sure to set IOTA_NOTARIZATION_PKG_ID environment variable

if [ -z "$IOTA_NOTARIZATION_PKG_ID" ]; then
    echo "Error: IOTA_NOTARIZATION_PKG_ID environment variable is not set"
    echo "Usage: IOTA_NOTARIZATION_PKG_ID=0x... ./run.sh"
    exit 1
fi

echo "Running all notarization examples..."
echo "Package ID: $IOTA_NOTARIZATION_PKG_ID"
echo "================================"

examples=(
    "01_create_locked_notarization"
    "02_create_dynamic_notarization"
    "03_update_dynamic_notarization"
    "04_destroy_notarization"
    "05_update_state"
    "06_update_metadata"
    "07_transfer_dynamic_notarization"
    "08_access_read_only_methods"
)

for example in "${examples[@]}"; do
    echo ""
    echo "Running: $example"
    echo "------------------------"
    cargo run --release --example "$example"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to run $example"
        exit 1
    fi
done

echo ""
echo "All examples completed successfully!"
