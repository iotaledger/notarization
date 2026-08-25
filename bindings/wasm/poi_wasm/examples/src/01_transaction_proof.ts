// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify a Transaction Proof
 *
 * Creates an IOTA transaction, constructs its Proof of Inclusion, serializes
 * the proof for transfer, and verifies it using the configured committee trust
 * model.
 *
 * ## Actors
 *
 * - **Ledger source**: Supplies untrusted transaction, checkpoint, and
 *   committee-transition evidence.
 * - **Prover application**: Selects the transaction and constructs a portable proof.
 * - **Verifier application**: Starts from a trusted genesis blob and verifies
 *   the proof locally.
 *
 * The endpoint supplies evidence; the genesis blob establishes trust.
 */

import { strict as assert } from "node:assert";

import { fromBase58 } from "@iota/iota-sdk/utils";
import { Proof } from "@iota/poi-wasm";
import { createNotarization, loadGenesisCommitteeResolution, preparePoiExample } from "./util.js";

/** Demonstrates how to construct, transfer, and verify a transaction proof. */
export async function createAndVerifyTransactionProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify a Transaction Proof ===\n");

    console.log("Stage 1 - Configure the proof source and establish committee trust");
    const context = await preparePoiExample();
    const trust = await loadGenesisCommitteeResolution(context);
    console.log(`  trust anchor: ${trust.description}`);

    console.log("\nStage 2 - Create a Notarization object using Locked Notarization");
    const targets = await createNotarization(context);
    const transaction = fromBase58(targets.transactionDigest);

    // Only the transaction is selected even though the context also created an
    // object and emitted an event.
    console.log("\nStage 3 - Construct a proof for the creation transaction");
    const proof = await context.poiClient.makeProof({ transaction });
    const proofTargets = proof.targets;

    assert.equal(proofTargets.transaction, targets.transactionDigest);
    assert.equal(proofTargets.objects.length, 0);
    assert.equal(proofTargets.events.length, 0);

    console.log("  proof constructed:");
    console.log(`    format version:   ${proof.version}`);
    console.log(`    checkpoint epoch: ${proof.checkpointEpoch}\n`);

    // Model transfer through a file, API, message, or process boundary.
    // The payload remains untrusted until verification succeeds.
    console.log("Stage 4 - Serialize and receive the untrusted proof");
    const proofJSON = proof.toJSON();
    const receivedProof = Proof.fromJSON(proofJSON);
    assert.equal(receivedProof.version, proof.version);
    assert.equal(receivedProof.checkpointEpoch, proof.checkpointEpoch);
    console.log(`  serialized proof size: ${Buffer.byteLength(proofJSON)} bytes\n`);

    console.log("Stage 5 - Verify the received proof");
    const verifier = context.poiClient.verifier(trust.resolution);
    await verifier.verify(receivedProof);

    console.log("  transaction proof verified successfully.");
    console.log(`The transaction is included in a checkpoint authenticated through ${trust.description}.`);
}
