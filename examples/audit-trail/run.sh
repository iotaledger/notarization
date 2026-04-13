#!/bin/bash

# Script to run all audit trail examples
# Usage: ./run.sh
# Make sure to set IOTA_AUDIT_TRAIL_PKG_ID and IOTA_TF_COMPONENTS_PKG_ID environment variables

if [[ -z $IOTA_AUDIT_TRAIL_PKG_ID || -z $IOTA_TF_COMPONENTS_PKG_ID ]]; then
    echo "Error: IOTA_AUDIT_TRAIL_PKG_ID environment variable is not set"
    echo "Usage: IOTA_AUDIT_TRAIL_PKG_ID=0x... IOTA_TF_COMPONENTS_PKG_ID=0x... ./run.sh"
    echo ""
    echo "On localnet, you can set both variables using:"
    echo "  eval \$(./audit-trail-move/scripts/publish_package.sh)"
    exit 1
fi

echo "Running all audit trail examples..."
echo "AuditTrail Package ID: $IOTA_AUDIT_TRAIL_PKG_ID"
echo "TfComponents Package ID: $IOTA_TF_COMPONENTS_PKG_ID"
echo "================================"

examples=(
    "01_create_audit_trail"
    "02_add_and_read_records"
    "03_update_metadata"
    "04_configure_locking"
    "05_manage_access"
    "06_delete_records"
    "07_access_read_only_methods"
    "08_delete_audit_trail"
    "09_tagged_records"
    "10_capability_constraints"
    "11_manage_record_tags"
    "01_customs_clearance"
    "02_clinical_trial"
)

for example in "${examples[@]}"; do
    echo ""
    echo "Running Audit Trail: $example"
    echo "------------------------"
    cargo run --release --example "$example"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to run $example"
        exit 1
    fi
done

echo ""
echo "All Audit Trail examples completed successfully!"
