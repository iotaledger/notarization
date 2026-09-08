#!/bin/bash

# Copyright 2020-2026 IOTA Stiftung
# SPDX-License-Identifier: Apache-2.0

set -eu

script_dir=$(cd "$(dirname "$0")" && pwd)
package_dir="$script_dir/.."
chain_id=$(iota client chain-identifier --json | jq -r '.')

case "$chain_id" in
6364aad5 | 2304aa97 | daf90477)
    response=$(iota client publish \
        --silence-warnings \
        --json \
        --gas-budget 500000000 \
        "$package_dir")
    ;;
*)
    response=$(iota client publish \
        --with-unpublished-dependencies \
        --silence-warnings \
        --json \
        --gas-budget 500000000 \
        "$package_dir")
    ;;
esac
package_id=$(
    echo "$response" | jq -r '
        .objectChanges[]
        | select(.type == "published")
        | .packageId
    '
)

if [ -z "$package_id" ] || [ "$package_id" = "null" ]; then
    echo "$response" >&2
    echo "failed to extract the IotaNotarization package ID from the publish response" >&2
    exit 1
fi

export IOTA_NOTARIZATION_PKG_ID="$package_id"
printf '%s\n' "$IOTA_NOTARIZATION_PKG_ID"
