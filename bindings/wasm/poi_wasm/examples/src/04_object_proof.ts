// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify an Object Proof
 *
 * Starting from only an object ID, the builder fetches the object's latest
 * version, discovers the transaction that produced it, and packages the object
 * and transaction evidence into one proof.
 *
 * The discovered transaction supports the object claim but does not become an
 * explicit transaction target. Trusted-node resolution keeps the example
 * focused on object-driven discovery.
 */

import { strict as assert } from "node:assert";

import { fromHex, normalizeIotaObjectId } from "@iota/iota-sdk/utils";
import { CommitteeResolution } from "@iota/poi-wasm";
import { createNotarization, preparePoiExample } from "./util.js";

/** Demonstrates target-driven object discovery and verification. */
export async function createAndVerifyObjectProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify an Object Proof ===\n");

    console.log("Stage 1 - Create a Notarization object using Locked Notarization");
    const context = await preparePoiExample();
    const targets = await createNotarization(context);

    // No transaction digest is supplied. The builder discovers the transaction
    // that produced the latest object version and constructs its evidence.
    console.log("\nStage 2 - Construct a proof from only the Notarization object ID");
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

    console.log("Stage 3 - Verify the object proof");
    await context.poiClient.verifier(CommitteeResolution.trustedNode()).verify(proof);

    console.log("  object proof verified successfully.");
    console.log("The resolved object version was changed by a transaction trusted through the selected node.");
}
