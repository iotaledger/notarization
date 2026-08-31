// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify an Event Proof
 *
 * An event ID contains its emitting transaction digest and sequence number, so
 * it can select an execution without a separate transaction target. The proof
 * carries the transaction's complete event list because the effects commit to
 * that complete list.
 *
 * Trusted-node resolution keeps the example focused on event-driven discovery.
 */

import { strict as assert } from "node:assert";

import { fromBase58 } from "@iota/iota-sdk/utils";
import { CommitteeResolution } from "@iota/poi-wasm";
import { createNotarization, preparePoiExample } from "./util.js";

/** Demonstrates event-driven transaction discovery and verification. */
export async function createAndVerifyEventProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify an Event Proof ===\n");

    console.log("Stage 1 - Emit a LockedNotarizationCreated event as fresh proof evidence");
    const context = await preparePoiExample();
    const targets = await createNotarization(context);
    const transaction = fromBase58(targets.transactionDigest);

    // The event ID already identifies the emitting transaction, allowing the
    // builder to fetch all required transaction, effects, and event evidence.
    console.log("\nStage 2 - Construct a proof from only the event ID");
    const proof = await context.poiClient.makeProof({
        events: [{ transaction, sequence: targets.eventSequence }],
    });
    const proofTargets = proof.targets;

    assert.equal(proofTargets.transaction, undefined);
    assert.equal(proofTargets.objects.length, 0);
    assert.equal(proofTargets.events.length, 1);
    assert.equal(proofTargets.events[0]?.transactionDigest, targets.transactionDigest);
    assert.equal(proofTargets.events[0]?.eventSequence, targets.eventSequence);

    console.log("  proof constructed:");
    console.log(`    checkpoint epoch: ${proof.checkpointEpoch}`);
    console.log(`    event targets:    ${proofTargets.events.length}\n`);

    console.log("Stage 3 - Verify the event proof");
    const verified = await context.poiClient.verifier(CommitteeResolution.trustedNode()).verify(proof);

    console.log("  event proof verified successfully.");
    console.log(
        `  authenticated event: ${verified.targets.events[0]?.transactionDigest}:${verified.targets.events[0]?.eventSequence}`,
    );
    console.log(`  event contents: ${verified.eventContents(0).length} BCS bytes`);
    console.log("The selected event was emitted by a transaction trusted through the selected node.");
}
