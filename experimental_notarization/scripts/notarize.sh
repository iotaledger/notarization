#!/bin/bash
set -euo pipefail

# ===== Configuration =====
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACT_DIR="$CURRENT_DIR/../packages/iota_notarization"
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
    local transfer_lock="$4"

    echo "Creating dynamic notarization..."
    echo "Data: $data"
    echo "Metadata: $metadata"
    echo "Description: $description"

    # Handle optional transfer lock
    local transfer_lock_cmd=""
    if [ -n "$transfer_lock" ] && [ "$transfer_lock" != "none" ]; then
        echo "Transfer lock: $transfer_lock"
        transfer_lock_cmd="--move-call \"$PACKAGE_ADDRESS::timelock::new_unlock_at\" $transfer_lock \"$CLOCK_ADDRESS\" --assign transfer_lock"
        transfer_lock_param="--move-call std::option::some \"<${PACKAGE_ADDRESS}::timelock::TimeLock>\" transfer_lock"
    else
        transfer_lock_param="--move-call std::option::none \"<${PACKAGE_ADDRESS}::timelock::TimeLock>\""
    fi

    # Build and execute the transaction
    cmd="iota client ptb \
        --make-move-vec \"<u8>\" \"$data\" \
        --assign state_data \
        --move-call std::option::some \"<std::string::String>\" \'$description\' \
        --assign description \
        --move-call \"$PACKAGE_ADDRESS::notarization::new_state_from_vector\" \
        state_data \'$metadata\' \
        --assign move_call_state \
        $transfer_lock_cmd \
        $transfer_lock_param \
        --assign transfer_lock_option \
        --move-call \"$PACKAGE_ADDRESS::dynamic_notarization::create\" \
        \"<vector<u8>>\" \
        move_call_state \
        description \
        \'$metadata\' \
        transfer_lock_option \
        \"$CLOCK_ADDRESS\" \
        --gas-budget \"$GAS_BUDGET\""

    # Remove any duplicate whitespace for cleaner command
    cmd=$(echo "$cmd" | tr -s ' ')
    eval "$cmd"
}

create_locked_notarization() {
    local data="$1"
    local metadata="$2"
    local description="$3"
    local delete_lock="$4"

    echo "Creating locked notarization..."
    echo "Data: $data"
    echo "Metadata: $metadata"
    echo "Description: $description"
    echo "Delete lock: $delete_lock"

    # Build the delete lock
    local delete_lock_cmd=""
    if [ "$delete_lock" == "until_destroyed" ]; then
        delete_lock_cmd="--move-call \"$PACKAGE_ADDRESS::timelock::until_destroyed\" --assign delete_lock"
    else
        delete_lock_cmd="--move-call \"$PACKAGE_ADDRESS::timelock::new_unlock_at\" $delete_lock \"$CLOCK_ADDRESS\" --assign delete_lock"
    fi

    # Build and execute the transaction
    cmd="iota client ptb \
        --make-move-vec \"<u8>\" \"$data\" \
        --assign state_data \
        --move-call std::option::some \"<std::string::String>\" \'$description\' \
        --assign description \
        --move-call \"$PACKAGE_ADDRESS::notarization::new_state_from_vector\" \
        state_data \'$metadata\' \
        --assign move_call_state \
        $delete_lock_cmd \
        --move-call \"$PACKAGE_ADDRESS::locked_notarization::create\" \
        \"<vector<u8>>\" \
        move_call_state \
        description \
        \'$metadata\' \
        delete_lock \
        \"$CLOCK_ADDRESS\" \
        --gas-budget \"$GAS_BUDGET\""

    # Remove any duplicate whitespace for cleaner command
    cmd=$(echo "$cmd" | tr -s ' ')
    eval "$cmd"
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
        --move-call "$PACKAGE_ADDRESS::notarization::new_state_from_vector" \
        new_state_data "'$new_metadata'" \
        --assign new_state \
        --move-call "$PACKAGE_ADDRESS::notarization::update_state" \
        "<vector<u8>>" \
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
        --type-args "vector<u8>" \
        --args "$notarization_id" "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

transfer_notarization() {
    local notarization_id="$1"
    local recipient="$2"

    echo "Transferring notarization: $notarization_id to $recipient"
    iota client call \
        --package "$PACKAGE_ADDRESS" \
        --module dynamic_notarization \
        --function transfer \
        --type-args "vector<u8>" \
        --args "$notarization_id" "$recipient" "$CLOCK_ADDRESS" \
        --gas-budget "$GAS_BUDGET"
}

usage() {
    echo "Usage: $0 <command> [arguments]"
    echo
    echo "Commands:"
    echo "  publish                                                  Publish the contract"
    echo "  create-dynamic <data> <metadata> <desc> [transfer_lock]  Create a dynamic notarization"
    echo "                                                          [transfer_lock] is optional Unix timestamp or 'none'"
    echo "  create-locked <data> <metadata> <desc> <delete_lock>     Create a locked notarization"
    echo "                                                          <delete_lock> is Unix timestamp or 'until_destroyed'"
    echo "  update <id> <new_data> <new_metadata>                   Update notarization state"
    echo "  destroy <id>                                            Destroy a notarization"
    echo "  transfer <id> <recipient>                               Transfer a dynamic notarization"
    echo
    echo "Examples:"
    echo "  $0 create-dynamic '[1,2,3]' 'Test data' 'My notarization'"
    echo "  $0 create-dynamic '[1,2,3]' 'Test data' 'My notarization' 2051218800"
    echo "  $0 create-locked '[1,2,3]' 'Test data' 'Locked notarization' 2051218800"
    echo "  $0 create-locked '[1,2,3]' 'Test data' 'Locked notarization' until_destroyed"
    echo "  $0 update 0x123...abc '[4,5,6]' 'Updated data'"
    echo "  $0 transfer 0x123...abc 0x456...def"
}

case "${1:-}" in
publish)
    publish_contract
    ;;
create-dynamic)
    if [ $# -lt 4 ]; then
        echo "Error: create-dynamic requires at least 3 arguments: <data> <metadata> <description>"
        exit 1
    fi
    transfer_lock="${5:-none}"
    create_dynamic_notarization "$2" "$3" "$4" "$transfer_lock"
    ;;
create-locked)
    if [ $# -ne 5 ]; then
        echo "Error: create-locked requires 4 arguments: <data> <metadata> <description> <delete_lock>"
        exit 1
    fi
    create_locked_notarization "$2" "$3" "$4" "$5"
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
transfer)
    if [ $# -ne 3 ]; then
        echo "Error: transfer requires 2 arguments: <id> <recipient>"
        exit 1
    fi
    transfer_notarization "$2" "$3"
    ;;
*)
    usage
    exit 1
    ;;
esac
