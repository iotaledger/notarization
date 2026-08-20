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
import { CommitteeResolution, PoiClient, Proof } from "@iota/poi-wasm";
import {
    loadMainnetGenesis,
    MAINNET_TRANSACTION_DIGEST,
    STAKED_IOTA_OBJECT_ID,
    STAKING_REQUEST_EVENT_SEQUENCE,
} from "./util.js";

/** Demonstrates how to construct and verify one proof containing three related targets. */
export async function createAndVerifyMultiTargetProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify a Multi-Target Proof ===\n");

    const client = PoiClient.mainnet();
    const transaction = fromBase58(MAINNET_TRANSACTION_DIGEST);
    const object = fromHex(normalizeIotaObjectId(STAKED_IOTA_OBJECT_ID, false, true));

    console.log("Network:            mainnet");
    console.log(`Transaction target: ${MAINNET_TRANSACTION_DIGEST}`);
    console.log(`Object target:      ${STAKED_IOTA_OBJECT_ID}`);
    console.log(`Event target:       ${MAINNET_TRANSACTION_DIGEST}:${STAKING_REQUEST_EVENT_SEQUENCE}\n`);

    // The explicit transaction and event identify one execution. The object ID
    // is resolved at the exact version recorded in that transaction's effects.
    const proof = await client
        .proof()
        .transaction(transaction)
        .object(object)
        .event(transaction, STAKING_REQUEST_EVENT_SEQUENCE)
        .build();
    const targets = proof.targets;

    assert.equal(targets.transaction, MAINNET_TRANSACTION_DIGEST);
    assert.equal(targets.objects.length, 1);
    assert.equal(targets.objects[0]?.objectId, STAKED_IOTA_OBJECT_ID);
    assert.equal(targets.events.length, 1);
    assert.equal(targets.events[0]?.transactionDigest, MAINNET_TRANSACTION_DIGEST);
    assert.equal(targets.events[0]?.eventSequence, STAKING_REQUEST_EVENT_SEQUENCE);

    console.log("Proof constructed:");
    console.log(`  checkpoint epoch: ${proof.checkpointEpoch}`);
    console.log(`  transaction targets: ${targets.transaction ? 1 : 0}`);
    console.log(`  object targets:      ${targets.objects.length}`);
    console.log(`  event targets:       ${targets.events.length}\n`);

    // Model transfer before verification. The receiver treats the entire payload
    // as untrusted input until every selected target verifies.
    const proofJSON = proof.toJSON();
    const receivedProof = Proof.fromJSON(proofJSON);
    console.log(`Serialized proof size: ${Buffer.byteLength(proofJSON)} bytes`);

    const genesis = await loadMainnetGenesis();
    const verifier = client.verifier(CommitteeResolution.fromGenesis(genesis));

    // Authenticating committee lineage through epoch 469 can take a long time.
    await verifier.verify(receivedProof);

    console.log("\nMulti-target proof verified successfully.");
    console.log("The transaction, changed object, and emitted event are authenticated by one proof.");
}
