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
./examples/notarization/run.sh
