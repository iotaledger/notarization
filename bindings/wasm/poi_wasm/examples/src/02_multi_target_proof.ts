// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify a Multi-Target Proof
 *
 * A single Proof of Inclusion can authenticate several claims about the same
 * transaction. This example proves the transaction itself, one object changed
 * by it, and one event emitted by it.
 *
 * Combining related targets avoids duplicating transaction and checkpoint
 * evidence. Every object and event target must belong to the same transaction.
 */

import { strict as assert } from "node:assert";

import { fromBase58, fromHex, normalizeIotaObjectId } from "@iota/iota-sdk/utils";
import { createNotarization, loadGenesisCommitteeResolution, preparePoiExample } from "./util.js";

/** Demonstrates how to construct and verify one proof containing three related targets. */
export async function createAndVerifyMultiTargetProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify a Multi-Target Proof ===\n");

    console.log("Stage 1 - Configure the proof source and establish committee trust");
    const context = await preparePoiExample();
    const trust = await loadGenesisCommitteeResolution(context);
    console.log(`  trust anchor: ${trust.description}`);

    console.log("\nStage 2 - Create one transaction with three related proof targets");
    const targets = await createNotarization(context);
    const transaction = fromBase58(targets.transactionDigest);
    const object = fromHex(normalizeIotaObjectId(targets.objectId, false, true));

    // The explicit transaction and event identify one execution. The object ID
    // is resolved at the exact version recorded in that transaction's effects.
    console.log("\nStage 3 - Construct one proof for the transaction, object, and event");
    const proof = await context.poiClient.makeProof({
        transaction,
        objects: [object],
        events: [{ transaction, sequence: targets.eventSequence }],
    });
    assert.equal(proof.targets.transaction, targets.transactionDigest);
    assert.equal(proof.targets.objects.length, 1);
    assert.equal(proof.targets.objects[0]?.objectId, targets.objectId);
    assert.equal(proof.targets.events.length, 1);
    assert.equal(proof.targets.events[0]?.transactionDigest, targets.transactionDigest);
    assert.equal(proof.targets.events[0]?.eventSequence, targets.eventSequence);

    console.log("  proof constructed:");
    console.log(`    checkpoint epoch:    ${proof.checkpointEpoch}`);
    console.log(`    transaction targets: ${proof.targets.transaction ? 1 : 0}`);
    console.log(`    object targets:      ${proof.targets.objects.length}`);
    console.log(`    event targets:       ${proof.targets.events.length}\n`);

    console.log("Stage 4 - Verify every target in the proof");
    const verifier = context.poiClient.verifier(trust.resolution);
    await verifier.verify(proof);

    console.log("  multi-target proof verified successfully.");
    console.log("The transaction, changed object, and emitted event are authenticated by one proof.");
}
