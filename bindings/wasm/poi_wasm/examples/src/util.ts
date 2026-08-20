// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";

const MAINNET_GENESIS_URL = "https://dbfiles.iota.org/genesis/mainnet/genesis.blob";

export const MAINNET_TRANSACTION_DIGEST = "86EvhdjqBb6Rt5pB8sKjTnE7MrzpNLuWTH3SuELBjDvu";
export const SECOND_MAINNET_TRANSACTION_DIGEST = "G8hfzqq9tCSEHF4cq9NMCZyKemuShmJoqfDDoG4K3C6z";
export const STAKED_IOTA_OBJECT_ID =
    "0x001270619f0ff6c5fce1925838a132241c73b9756dae9d0dcfab71bb03549f73";
export const STAKING_REQUEST_EVENT_SEQUENCE = 0n;

/** Downloads the trusted mainnet genesis blob once and reuses the local copy. */
export async function loadMainnetGenesis(): Promise<Uint8Array> {
    const cachePath = join(homedir(), ".iota", "iota_config", "poi", "mainnet", "genesis.blob");

    try {
        return await readFile(cachePath);
    } catch (error) {
        if (!isNodeError(error) || error.code !== "ENOENT") {
            throw error;
        }
    }

    console.log(`Downloading trusted mainnet genesis from ${MAINNET_GENESIS_URL}`);
    const response = await fetch(MAINNET_GENESIS_URL);
    if (!response.ok) {
        throw new Error(`failed to download mainnet genesis: ${response.status} ${response.statusText}`);
    }

    const genesis = new Uint8Array(await response.arrayBuffer());
    await mkdir(dirname(cachePath), { recursive: true });
    await writeFile(cachePath, genesis);
    return genesis;
}

/** Returns the elapsed milliseconds since a `process.hrtime.bigint()` reading. */
export function elapsedMilliseconds(start: bigint): number {
    return Number(process.hrtime.bigint() - start) / 1_000_000;
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
    return error instanceof Error && "code" in error;
}
