#!/bin/bash

# Copyright 2020-2026 IOTA Stiftung
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
package_dir="$script_dir/.."

active_env=$(iota client active-env --json | jq -r '.')
chain_id=$(iota client chain-identifier)

current_pkg_id=$(
    toml get "$package_dir/Move.lock" env \
        | jq -r --arg chain_id "$chain_id" '
            map(values | select(."chain-id" == $chain_id) ."latest-published-id")
            | first
        '
)

if [[ -z "$current_pkg_id" || "$current_pkg_id" == "null" ]]; then
    echo "failed to find current Audit Trails package ID for chain $chain_id in Move.lock" >&2
    exit 1
fi

upgrade_cap_id=$(
    iota client objects --json \
        | jq -r --arg current_pkg_id "$current_pkg_id" '
            map(
                select(
                    .data.type == "0x2::package::UpgradeCap"
                    and .data.content.fields.package == $current_pkg_id
                )
            )
            | first
            | .data.objectId
        '
)

if [[ -z "$upgrade_cap_id" || "$upgrade_cap_id" == "null" ]]; then
    echo "failed to find UpgradeCap for Audit Trails package $current_pkg_id" >&2
    exit 1
fi

upgrade_args=(
    iota client upgrade
    --upgrade-capability "$upgrade_cap_id"
    --verify-compatibility
    --silence-warnings
    --json
    --gas-budget 500000000
)

if [[ "$active_env" == "localnet" ]]; then
    upgrade_args+=(--with-unpublished-dependencies)
fi

echo "upgrading Audit Trails package $current_pkg_id using UpgradeCap $upgrade_cap_id" >&2

response=$("${upgrade_args[@]}" "$package_dir")

audit_trail_package_id=$(
    echo "$response" | jq -r '
        .objectChanges[]
        | select(.type == "published")
        | .packageId
    '
)

if [[ -z "$audit_trail_package_id" || "$audit_trail_package_id" == "null" ]]; then
    echo "$response" >&2
    echo "failed to extract upgraded Audit Trails package ID from upgrade response" >&2
    exit 1
fi

export IOTA_AUDIT_TRAIL_PKG_ID="$audit_trail_package_id"
printf 'export IOTA_AUDIT_TRAIL_PKG_ID=%s\n' "$IOTA_AUDIT_TRAIL_PKG_ID"

if [[ "$active_env" == "localnet" ]]; then
    tf_components_package_id="$audit_trail_package_id"

    export IOTA_TF_COMPONENTS_PKG_ID="$tf_components_package_id"
    printf 'export IOTA_TF_COMPONENTS_PKG_ID=%s\n' "$IOTA_TF_COMPONENTS_PKG_ID"
fi
