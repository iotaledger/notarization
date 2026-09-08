// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Verify a Proof Using a Trusted Node
 *
 * Trusted-node committee resolution accepts the committee reported by the
 * connected node without authenticating its lineage from genesis. It is
 * appropriate only when that node is inside the verifier's trust boundary.
 *
 * This example can run against any network, but the selected gRPC endpoint
 * must be operated by a party the verifier trusts.
 */

import { fromBase58 } from "@iota/iota-sdk/utils";
import { CommitteeResolution } from "@iota/proof-of-inclusion";
import { createNotarization, preparePoiExample } from "./util.js";

/** Demonstrates trusted-node committee resolution against a trusted endpoint. */
export async function verifyWithTrustedNode(): Promise<void> {
    console.log("=== Proof of Inclusion Advanced: Trusted-Node Resolution ===\n");

    const context = await preparePoiExample();

    const targets = await createNotarization(context, "PoI trusted-node example");
    const transaction = fromBase58(targets.transactionDigest);
    const proof = await context.poiClient.makeProof({ transaction });

    console.log(`Network:              ${context.networkAlias}`);
    console.log("Committee resolution: trusted node");
    console.log(`Transaction target:   ${targets.transactionDigest}\n`);

    const verified = await context.poiClient.verifier(CommitteeResolution.trustedNode()).verify(proof);

    console.log("Transaction proof verified successfully.");
    console.log(`  authenticated checkpoint: ${verified.checkpointSequenceNumber}`);
    console.log(`  authenticated transaction: ${verified.transaction}`);
    console.log("The selected node supplied the committee and is part of the trust boundary.");
}
