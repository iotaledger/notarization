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
 * explicit transaction target. Trusted-node resolution keeps this focused
 * example fast and is appropriate only when the node is inside the trust boundary.
 */

import { strict as assert } from "node:assert";

import { fromHex, normalizeIotaObjectId } from "@iota/iota-sdk/utils";
import { CommitteeResolution, PoiClient } from "@iota/poi-wasm";
import { STAKED_IOTA_OBJECT_ID } from "./util.js";

/** Demonstrates target-driven object discovery and trusted-node verification. */
export async function createAndVerifyObjectProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify an Object Proof ===\n");

    const client = PoiClient.mainnet();

    console.log("Network:       mainnet");
    console.log(`Object target: ${STAKED_IOTA_OBJECT_ID}\n`);

    // No transaction digest is supplied. The builder discovers the transaction
    // that produced the latest object version and constructs its evidence.
    const object = fromHex(normalizeIotaObjectId(STAKED_IOTA_OBJECT_ID, false, true));
    const proof = await client.proof().object(object).build();
    const targets = proof.targets;

    assert.equal(targets.transaction, undefined);
    assert.equal(targets.objects.length, 1);
    assert.equal(targets.objects[0]?.objectId, STAKED_IOTA_OBJECT_ID);
    assert.equal(targets.events.length, 0);

    console.log("Proof constructed:");
    console.log(`  checkpoint epoch: ${proof.checkpointEpoch}`);
    console.log(`  object version:   ${targets.objects[0]?.version}`);
    console.log(`  object targets:   ${targets.objects.length}\n`);

    // This avoids the genesis committee walk but places the selected node inside the trust boundary.
    await client.verifier(CommitteeResolution.trustedNode()).verify(proof);

    console.log("Object proof verified successfully.");
    console.log("The resolved object version was changed by a transaction in the verified checkpoint.");
}
