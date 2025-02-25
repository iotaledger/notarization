#!/bin/bash
set -euo pipefail

# ===== Configuration =====
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACT_DIR="$CURRENT_DIR/../packages/iota_notarization"
CONTRACT_PATH="$CONTRACT_DIR/sources/notarization.move"
GAS_BUDGET=500000000

# Package address of the notarization module (update after publishing)
PACKAGE_ADDRESS="0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380"
CLOCK_ADDRESS="@0x6" # Special address for the clock module

# ===== Core Functions =====
publish_contract() {
    echo "Publishing contract from: $CONTRACT_DIR"
    iota client publish "$CONTRACT_DIR" \
        --skip-dependency-verification \
        --gas-budget "$GAS_BUDGET"
    echo "Contract published successfully."
    echo "IMPORTANT: Update PACKAGE_ADDRESS in this script with the new package address"
}

create_dynamic_notarization() {
    local data="$1"
    local metadata="$2"
    local description="$3"

    echo "Creating dynamic notarization..."
    echo "Data: $data"
    echo "Metadata: $metadata"
    echo "Description: $description"

    iota client ptb \
        --make-move-vec "<u8>" "$data" \
        --assign state_data \
        --move-call std::option::some "<std::string::String>" "'$description'" \
        --assign description \
        --move-call "$PACKAGE_ADDRESS::notarization::new_default_state" \
        state_data "'$metadata'" \
        --assign move_call_state \
        --move-call "$PACKAGE_ADDRESS::notarization::create_dynamic_notarization" \
        "<${PACKAGE_ADDRESS}::notarization::DefaultState>" \
        move_call_state \
        description \
        "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

create_locked_notarization() {
    local data="$1"
    local metadata="$2"
    local description="$3"
    local update_lock="$4"
    local delete_lock="$5"

    echo "Creating locked notarization..."
    echo "Data: $data"
    echo "Metadata: $metadata"
    echo "Description: $description"
    echo "Update lock period: $update_lock"
    echo "Delete lock period: $delete_lock"

    iota client ptb \
        --make-move-vec "<u8>" "$data" \
        --assign state_data \
        --move-call std::option::some "<std::string::String>" "'$description'" \
        --assign description \
        --move-call "$PACKAGE_ADDRESS::notarization::new_default_state" \
        state_data "'$metadata'" \
        --assign move_call_state \
        --move-call "$PACKAGE_ADDRESS::lock_configuration::new_lock_configuration" \
        "$update_lock" "$delete_lock" \
        --assign lock_config \
        --move-call "$PACKAGE_ADDRESS::notarization::create_locked_notarization" \
        "<${PACKAGE_ADDRESS}::notarization::DefaultState>" \
        move_call_state \
        description \
        lock_config \
        "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

update_state() {
    local notarization_id="$1"
    local new_data="$2"
    local new_metadata="$3"


    echo "Updating notarization state..."
    echo "Notarization ID: $notarization_id"
    echo "New data: $new_data"
    echo "New metadata: $new_metadata"

    iota client ptb \
        --make-move-vec "<u8>" "$new_data" \
        --assign new_state_data \
        --move-call "$PACKAGE_ADDRESS::notarization::new_default_state" \
        new_state_data "'$new_metadata'" \
        --assign new_state \
        --move-call "$PACKAGE_ADDRESS::notarization::update_state" \
        "<${PACKAGE_ADDRESS}::notarization::DefaultState>" \
        "@$notarization_id" \
        new_state \
        "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

destroy_notarization() {
    local notarization_id="$1"

    echo "Destroying notarization: $notarization_id"
    iota client call \
        --package "$PACKAGE_ADDRESS" \
        --module notarization \
        --function destroy \
        --type-args "${PACKAGE_ADDRESS}::notarization::DefaultState" \
        --args "$notarization_id" "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

usage() {
    echo "Usage: $0 <command> [arguments]"
    echo
    echo "Commands:"
    echo "  publish                                    Publish the contract"
    echo "  create-dynamic <data> <metadata> <desc>    Create a dynamic notarization"
    echo "  create-locked <data> <metadata> <desc> <update_lock> <delete_lock>"
    echo "                                            Create a locked notarization"
    echo "  update <id> <new_data> <new_metadata>     Update notarization state"
    echo "  destroy <id>                              Destroy a notarization"
    echo
    echo "Examples:"
    echo "  $0 create-dynamic '[1,2,3]' 'Test data' 'My notarization'"
    echo "  $0 create-locked '[1,2,3]' 'Test data' 'Locked notarization' 2051218800 2051219000"
    echo "  $0 update 0x123...abc '[4,5,6]' 'Updated data'"
}

case "${1:-}" in
publish)
    publish_contract
    ;;
create-dynamic)
    if [ $# -ne 4 ]; then
        echo "Error: create-dynamic requires 3 arguments: <data> <metadata> <description>"
        exit 1
    fi
    create_dynamic_notarization "$2" "$3" "$4"
    ;;
create-locked)
    if [ $# -ne 6 ]; then
        echo "Error: create-locked requires 5 arguments: <data> <metadata> <description> <update_lock> <delete_lock>"
        exit 1
    fi
    create_locked_notarization "$2" "$3" "$4" "$5" "$6"
    ;;
update)
    if [ $# -ne 4 ]; then
        echo "Error: update requires 3 arguments: <id> <new_data> <new_metadata>"
        exit 1
    fi
    update_state "$2" "$3" "$4"
    ;;
destroy)
    if [ $# -ne 2 ]; then
        echo "Error: destroy requires 1 argument: <id>"
        exit 1
    fi
    destroy_notarization "$2"
    ;;
*)
    usage
    exit 1
    ;;
esac
