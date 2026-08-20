// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Reuse a Verifier for Multiple Proofs
 *
 * Genesis-anchored verification authenticates every committee transition from
 * epoch 0 to the proof's checkpoint epoch. A verifier retains authenticated
 * committees in memory, so applications should reuse it across proofs from the same network.
 */

import { strict as assert } from "node:assert";

import { fromBase58 } from "@iota/iota-sdk/utils";
import { CommitteeResolution, PoiClient } from "@iota/poi-wasm";
import {
    elapsedMilliseconds,
    loadMainnetGenesis,
    MAINNET_TRANSACTION_DIGEST,
    SECOND_MAINNET_TRANSACTION_DIGEST,
} from "./util.js";

/** Demonstrates how one verifier avoids repeating an authenticated committee walk. */
export async function reuseVerifierForMultipleProofs(): Promise<void> {
    console.log("=== Proof of Inclusion: Reuse a Verifier for Multiple Proofs ===\n");

    const client = PoiClient.mainnet();
    const firstProof = await client
        .proof()
        .transaction(fromBase58(MAINNET_TRANSACTION_DIGEST))
        .build();
    const secondProof = await client
        .proof()
        .transaction(fromBase58(SECOND_MAINNET_TRANSACTION_DIGEST))
        .build();

    assert.equal(firstProof.checkpointEpoch, secondProof.checkpointEpoch);

    console.log(`First transaction:  ${MAINNET_TRANSACTION_DIGEST}`);
    console.log(`Second transaction: ${SECOND_MAINNET_TRANSACTION_DIGEST}`);
    console.log(`Checkpoint epoch:   ${firstProof.checkpointEpoch}\n`);

    const genesis = await loadMainnetGenesis();

    // Keep this verifier alive. Its default cache contains only committees
    // authenticated through the genesis-anchored epoch walk.
    const verifier = client.verifier(CommitteeResolution.fromGenesis(genesis));

    console.log("Verifying the first proof; this performs the epoch walk...");
    const firstStart = process.hrtime.bigint();
    await verifier.verify(firstProof);
    const firstDuration = elapsedMilliseconds(firstStart);

    console.log("Verifying the second proof with the same verifier...");
    const secondStart = process.hrtime.bigint();
    await verifier.verify(secondProof);
    const secondDuration = elapsedMilliseconds(secondStart);

    console.log("\nBoth transaction proofs verified successfully.");
    console.log(`First verification:  ${firstDuration.toFixed(2)} ms`);
    console.log(`Second verification: ${secondDuration.toFixed(2)} ms`);
    console.log("The second verification reused the authenticated epoch-469 committee.");
}
