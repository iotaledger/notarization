#!/bin/bash
set -euo pipefail

CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Configuration (adjust these variables as needed)
IOTA_CLIENT_BIN="iota" # Ensure this is in your PATH or include full path
CONTRACT_DIR="$CURRENT_DIR/../packages/iota_notarization"
CONTRACT_PATH="$CONTRACT_DIR/sources/notarization.move"

# After publishing the module, update MODULE_ADDRESS with its on-chain address
MODULE_ADDRESS="0xbbfad0bff63ebe02c6de79e2e3650c990a2b47fe93c63d2d5afb83d89c41ecb3"
WALLET="0xbd29669bb8ec4f388c1dc21db1bee8c3050c0c3d0f781de63845c434dfaa1024" # Replace with your wallet address
GAS_BUDGET=10000000

publish_contract() {
    echo "Publishing contract from: $CONTRACT_DIR"
    $IOTA_CLIENT_BIN client publish "$CONTRACT_DIR" \
        --json \
        --gas-budget "$GAS_BUDGET"
    echo "Contract published. Check the output for the module address and update MODULE_ADDRESS."
}

create_notarization() {
    local DESCRIPTION="[84, 101, 115, 116, 32, 110, 111, 116, 97, 114, 105, 122, 97, 116, 105, 111, 110]" # Test Notarization
    local STATE_DATA="[73, 68, 58, 32, 53, 53, 53]"                                                       # ID: 555 in UTF-8
    local METADATA="[73, 68]"                                                                             # ID

    echo "Creating a new notarization via PTB with description '$DESCRIPTION' and state data '$STATE_DATA' using Clock address 0x6"
    $IOTA_CLIENT_BIN client ptb \
        --make-move-vec "<u8>" $STATE_DATA \
        --assign state_data \
        --make-move-vec "<u8>" $DESCRIPTION \
        --assign description \
        --move-call "std::string::utf8" \
        description \
        --assign move_call_description \
        --make-move-vec "<u8>" $METADATA \
        --assign metadata \
        --move-call "std::string::utf8" \
        move_call_metadata \
        --move-call "$MODULE_ADDRESS::notarization::new_default_state" \
        state_data \
        move_call_metadata

    --assign move_call_state \
        --move-call "$MODULE_ADDRESS::notarization::create_and_transfer" \
        "<${MODULE_ADDRESS}::notarization::DefaultState>" \
        move_call_state \
        move_call_description \
        "0x6" \
        --gas-budget "$GAS_BUDGET"
    echo "PTB Notarization creation transaction submitted."
}

# destroy_empty() {
#     if [ -z "${1:-}" ]; then
#         echo "Usage: destroy_empty <notarization_id>"
#         exit 1
#     fi
#     local NOTARIZATION_ID="$1"
#     echo "Destroying empty notarization with id: $NOTARIZATION_ID"
#     $IOTA_CLIENT_BIN move call \
#         --package "$MODULE_ADDRESS" \
#         --module notarization \
#         --function destroy_empty \
#         --args "$NOTARIZATION_ID" \
#         --sender "$WALLET" \
#         --gas-budget "$GAS_BUDGET"
#     echo "Destroy transaction submitted."
# }

get_state() {
    if [ -z "${1:-}" ]; then
        echo "Usage: get_state <notarization_id>"
        exit 1
    fi
    local NOTARIZATION_ID="$1"
    echo "Querying state for notarization with id: $NOTARIZATION_ID"
    $IOTA_CLIENT_BIN move query \
        --package "$MODULE_ADDRESS" \
        --module notarization \
        --function state \
        --args "$NOTARIZATION_ID"
}

usage() {
    echo "Usage: $0 {publish|create|destroy|getstate} [arguments]"
    echo "Commands:"
    echo "  publish               Publish the contract"
    echo "  create                Create a new notarization"
    echo "  destroy <id>          Destroy an empty notarization with the specified id"
    echo "  getstate <id>         Query the state of a notarization with the specified id"
}

# Main script logic based on subcommand
case "${1:-}" in
publish)
    publish_contract
    ;;
create)
    create_notarization
    ;;
destroy)
    destroy_empty "${2:-}"
    ;;
getstate)
    get_state "${2:-}"
    ;;
*)
    usage
    exit 1
    ;;
esac
