#!/bin/bash

# Copyright 2020-2026 IOTA Stiftung
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
package_dir="$script_dir/.."

active_env=$(iota client active-env --json | jq -r '.')

publish_args=(
    iota client publish
    --silence-warnings
    --json
    --gas-budget 500000000
)

if [[ "$active_env" == "localnet" ]]; then
    publish_args+=(--with-unpublished-dependencies)
fi

response=$("${publish_args[@]}" "$package_dir")

audit_trail_package_id=$(
    echo "$response" | jq -r '
        .objectChanges[]
        | select(.type == "published")
        | .packageId
    '
)

if [[ -z "$audit_trail_package_id" || "$audit_trail_package_id" == "null" ]]; then
    echo "$response" >&2
    echo "failed to extract audit_trail package ID from publish response" >&2
    exit 1
fi

export IOTA_AUDIT_TRAIL_PKG_ID="$audit_trail_package_id"
printf 'export IOTA_AUDIT_TRAIL_PKG_ID=%s\n' "$IOTA_AUDIT_TRAIL_PKG_ID"

if [[ "$active_env" == "localnet" ]]; then
    tf_components_package_id="$audit_trail_package_id"

    export IOTA_TF_COMPONENTS_PKG_ID="$tf_components_package_id"
    printf 'export IOTA_TF_COMPONENTS_PKG_ID=%s\n' "$IOTA_TF_COMPONENTS_PKG_ID"
fi
