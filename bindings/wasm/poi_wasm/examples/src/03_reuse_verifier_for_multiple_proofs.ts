// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Reuse a Verifier for Multiple Proofs
 *
 * A verifier retains resolved committees in memory, so applications should
 * reuse it across proofs from the same network.
 */

import { fromBase58 } from "@iota/iota-sdk/utils";
import {
    createNotarization,
    elapsedMilliseconds,
    loadGenesisCommitteeResolution,
    preparePoiExample,
} from "./util.js";

/** Demonstrates how one verifier reuses committee resolution across proofs. */
export async function reuseVerifierForMultipleProofs(): Promise<void> {
    console.log("=== Proof of Inclusion: Reuse a Verifier for Multiple Proofs ===\n");

    console.log("Stage 1 - Configure the proof source and establish committee trust");
    const context = await preparePoiExample();
    const trust = await loadGenesisCommitteeResolution(context);
    console.log(`  trust anchor: ${trust.description}`);

    console.log("\nStage 2 - Create two Notarization objects using Locked Notarization");
    const firstTargets = await createNotarization(context, "First Notarization object");
    const secondTargets = await createNotarization(context, "Second Notarization object");

    console.log("\nStage 3 - Construct one transaction proof for each Notarization object");
    const firstProof = await context.poiClient.makeProof({
        transaction: fromBase58(firstTargets.transactionDigest),
    });
    const secondProof = await context.poiClient.makeProof({
        transaction: fromBase58(secondTargets.transactionDigest),
    });

    console.log(`  first proof epoch:  ${firstProof.checkpointEpoch}`);
    console.log(`  second proof epoch: ${secondProof.checkpointEpoch}\n`);

    // Keep this verifier alive so its committee cache can serve both proofs.
    const verifier = context.poiClient.verifier(trust.resolution);

    console.log("Stage 4 - Verify both proofs with one verifier");
    console.log("  verifying the first proof; this resolves its checkpoint committee...");
    const firstStart = process.hrtime.bigint();
    await verifier.verify(firstProof);
    const firstDuration = elapsedMilliseconds(firstStart);

    console.log("  verifying the second proof with the same verifier...");
    const secondStart = process.hrtime.bigint();
    await verifier.verify(secondProof);
    const secondDuration = elapsedMilliseconds(secondStart);

    console.log("\n  both transaction proofs verified successfully.");
    console.log(`  first verification:  ${firstDuration.toFixed(2)} ms`);
    console.log(`  second verification: ${secondDuration.toFixed(2)} ms`);
    console.log(`Both proofs used committee resolution through ${trust.description}.`);
}
