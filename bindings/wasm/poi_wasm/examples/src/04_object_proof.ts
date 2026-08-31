// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify an Object Proof
 *
 * Starting from only an object ID, the builder fetches its latest version at
 * proof construction time, discovers the transaction that produced it, and
 * packages both as one proof.
 *
 * The discovered transaction supports the object claim but does not become an
 * explicit transaction target. Verification authenticates committee history
 * from a trusted genesis blob.
 */

import { strict as assert } from "node:assert";

import { fromHex, normalizeIotaObjectId } from "@iota/iota-sdk/utils";
import { createNotarization, loadGenesisCommitteeResolution, preparePoiExample } from "./util.js";

/** Demonstrates target-driven object discovery and verification. */
export async function createAndVerifyObjectProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify an Object Proof ===\n");

    console.log("Stage 1 - Configure the proof source and establish committee trust");
    const context = await preparePoiExample();
    const trust = await loadGenesisCommitteeResolution(context);
    console.log(`  trust anchor: ${trust.description}`);

    console.log("\nStage 2 - Create a Notarization object using Locked Notarization");
    const targets = await createNotarization(context);

    // No transaction digest is supplied. The builder discovers the transaction
    // that produced the version returned at build time and constructs its evidence.
    console.log("\nStage 3 - Construct a proof from only the Notarization object ID");
    const object = fromHex(normalizeIotaObjectId(targets.objectId, false, true));
    const proof = await context.poiClient.makeProof({ objects: [object] });
    const proofTargets = proof.targets;

    assert.equal(proofTargets.transaction, undefined);
    assert.equal(proofTargets.objects.length, 1);
    assert.equal(proofTargets.objects[0]?.objectId, targets.objectId);
    assert.equal(proofTargets.events.length, 0);

    console.log("  proof constructed:");
    console.log(`    checkpoint epoch: ${proof.checkpointEpoch}`);
    console.log(`    object version:   ${proofTargets.objects[0]?.version}`);
    console.log(`    object targets:   ${proofTargets.objects.length}\n`);

    console.log("Stage 4 - Verify the object proof");
    const verified = await context.poiClient.verifier(trust.resolution).verify(proof);

    console.log("  object proof verified successfully.");
    console.log(
        `  authenticated object: ${verified.targets.objects[0]?.objectId} at version ${
            verified.targets.objects[0]?.version
        }`,
    );
    console.log(`  object BCS: ${verified.objectBcs(0).length} bytes`);
    console.log("The resolved object version was authenticated from the trusted network genesis.");
}
