// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Create and Verify a Transaction Proof
 *
 * Constructs a Proof of Inclusion for an existing IOTA transaction, serializes
 * the proof for transfer, and verifies it from a trusted genesis blob.
 *
 * ## Actors
 *
 * - **Ledger source**: Supplies untrusted transaction, checkpoint, and committee-transition evidence.
 * - **Prover application**: Selects the transaction and constructs a portable proof.
 * - **Verifier application**: Starts from an independently trusted genesis blob,
 *   authenticates the checkpoint committee, and verifies the proof locally.
 *
 * The endpoint supplies evidence; the genesis blob establishes trust.
 */

import { strict as assert } from "node:assert";

import { fromBase58 } from "@iota/iota-sdk/utils";
import { CommitteeResolution, PoiClient, Proof } from "@iota/poi-wasm";
import { loadMainnetGenesis, MAINNET_TRANSACTION_DIGEST } from "./util.js";

/** Demonstrates how to construct, transfer, and verify a transaction proof from mainnet genesis. */
export async function createAndVerifyTransactionProof(): Promise<void> {
    console.log("=== Proof of Inclusion: Create and Verify a Transaction Proof ===\n");

    const client = PoiClient.mainnet();
    const transaction = fromBase58(MAINNET_TRANSACTION_DIGEST);

    console.log("Network:            mainnet");
    console.log(`Transaction target: ${MAINNET_TRANSACTION_DIGEST}\n`);

    // Step 1: Build the proof from evidence supplied by the ledger source.
    // Selecting mainnet controls where evidence is fetched; it does not make that evidence trusted.
    const proof = await client.proof().transaction(transaction).build();
    const targets = proof.targets;

    assert.equal(targets.transaction, MAINNET_TRANSACTION_DIGEST);
    assert.equal(targets.objects.length, 0);
    assert.equal(targets.events.length, 0);

    console.log("Proof constructed:");
    console.log(`  format version:   ${proof.version}`);
    console.log(`  checkpoint epoch: ${proof.checkpointEpoch}\n`);

    // Step 2: Model transfer through a file, API, message, or process boundary.
    // The payload remains untrusted until verification succeeds.
    const proofJSON = proof.toJSON();
    const receivedProof = Proof.fromJSON(proofJSON);
    assert.equal(receivedProof.version, proof.version);
    assert.equal(receivedProof.checkpointEpoch, proof.checkpointEpoch);
    console.log(`Serialized proof size: ${Buffer.byteLength(proofJSON)} bytes`);

    // Step 3: Establish trust independently from the mainnet genesis blob.
    const genesis = await loadMainnetGenesis();
    const verifier = client.verifier(CommitteeResolution.fromGenesis(genesis));

    // This proof is from epoch 469. Authenticating every committee transition
    // from genesis can take a long time on the first run.
    await verifier.verify(receivedProof);

    console.log("\nTransaction proof verified successfully.");
    console.log("The transaction is included in a checkpoint authenticated from mainnet genesis.");
}
