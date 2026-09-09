#!/bin/bash

set -e

# Script to run all examples contained in this directory
# Usage: ./run.sh
# Make sure to set the following environment variables:
# - IOTA_NOTARIZATION_PKG_ID: The package ID of the notarization module
# - IOTA_AUDIT_TRAIL_PKG_ID: The package ID of the audit trail module
# - IOTA_TF_COMPONENTS_PKG_ID: The package ID of the tf components module
# - IOTA_GENESIS_PATH: The trusted genesis blob for PoI on local or custom networks
# PoI uses the active IOTA CLI environment and wallet; see examples/poi/README.md.

./examples/audit-trail/run.sh
printf "\n================================\n"
printf "================================\n\n"

# Script to run all notarization examples
# Usage: ./run.sh
# Make sure to set IOTA_NOTARIZATION_PKG_ID environment variable

if [ -z "$IOTA_NOTARIZATION_PKG_ID" ]; then
    echo "Error: IOTA_NOTARIZATION_PKG_ID environment variable is not set"
    echo "Usage: IOTA_NOTARIZATION_PKG_ID=0x... ./run.sh"
    exit 1
fi

echo "Running all Notarization examples..."
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
    "01_iot_weather_station"
    "02_legal_contract"
)

for example in "${examples[@]}"; do
    echo ""
    echo "Running Notarization Example: $example"
    echo "------------------------"
    cargo run --release --example "$example"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to run $example"
        exit 1
    fi
done

echo ""
echo "All Notarization examples completed successfully!"

./examples/poi/run.sh
