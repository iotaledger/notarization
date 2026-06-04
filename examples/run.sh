#!/bin/bash

# Script to run all examples contained in this directory
# Usage: ./run.sh
# Make sure to set the following environment variables:
# - IOTA_NOTARIZATION_PKG_ID: The package ID of the notarization module
# - IOTA_AUDIT_TRAIL_PKG_ID: The package ID of the audit trail module
# - IOTA_TF_COMPONENTS_PKG_ID: The package ID of the tf components module

./examples/audit-trail/run.sh
printf "\n================================\n"
printf "================================\n\n"

# At the moment the `examples` folder contains all single notarization (SN) examples.
# After a `notarization` folder has been introduced to contain the SN examples,
# uncomment the following line, update the paths in Cargo.toml and move the bash code below that line into `./examples/notarization/run.sh`.

# ./examples/notarization/run.sh

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
