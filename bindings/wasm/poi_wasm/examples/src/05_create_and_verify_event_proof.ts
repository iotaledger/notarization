// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify an Event Proof
 *
 * An event ID contains its emitting transaction digest and sequence number, so
 * it can select an execution without a separate transaction target. The proof
 * carries the transaction's complete event list because the effects commit to that complete list.
 *
 * Trusted-node resolution keeps this focused example fast and is appropriate
 * only when the node is inside the verifier's trust boundary.
 */

import { strict as assert } from "node:assert";

import { fromBase58 } from "@iota/iota-sdk/utils";
import { CommitteeResolution, PoiClient } from "@iota/poi-wasm";
import {
    MAINNET_TRANSACTION_DIGEST,
    STAKING_REQUEST_EVENT_SEQUENCE,
} from "./util.js";

/** Demonstrates event-driven transaction discovery and trusted-node verification. */
export async function createAndVerifyEventProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify an Event Proof ===\n");

    const client = PoiClient.mainnet();
    const transaction = fromBase58(MAINNET_TRANSACTION_DIGEST);

    console.log("Network:      mainnet");
    console.log(`Event target: ${MAINNET_TRANSACTION_DIGEST}:${STAKING_REQUEST_EVENT_SEQUENCE}\n`);

    // The event ID already identifies the emitting transaction, allowing the
    // builder to fetch all required transaction, effects, and event evidence.
    const proof = await client
        .proof()
        .event(transaction, STAKING_REQUEST_EVENT_SEQUENCE)
        .build();
    const targets = proof.targets;

    assert.equal(targets.transaction, undefined);
    assert.equal(targets.objects.length, 0);
    assert.equal(targets.events.length, 1);
    assert.equal(targets.events[0]?.transactionDigest, MAINNET_TRANSACTION_DIGEST);
    assert.equal(targets.events[0]?.eventSequence, STAKING_REQUEST_EVENT_SEQUENCE);

    console.log("Proof constructed:");
    console.log(`  checkpoint epoch: ${proof.checkpointEpoch}`);
    console.log(`  event targets:    ${targets.events.length}\n`);

    // This avoids the genesis committee walk but places the selected node inside the trust boundary.
    await client.verifier(CommitteeResolution.trustedNode()).verify(proof);

    console.log("Event proof verified successfully.");
    console.log("The selected event was emitted by a transaction in the verified checkpoint.");
}
