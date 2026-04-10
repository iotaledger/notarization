// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and holds the built-in Admin capability that is
 *   automatically minted on creation.
 * - **RecordAdmin**: Receives a RecordAdmin capability bound to their address. Writes
 *   records in subsequent examples.
 *
 * Demonstrates how to:
 * 1. Create an audit trail with immutable metadata, updatable metadata, and a seed record.
 * 2. Inspect the built-in Admin role.
 * 3. Define a RecordAdmin role and issue a capability for it.
 */

import { CapabilityIssueOptions, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function createAuditTrail(): Promise<void> {
    console.log("Creating an audit trail");

    // `admin` creates the trail and holds the Admin capability.
    // `recordAdmin` receives the RecordAdmin capability.
    const admin = await getFundedClient();
    const recordAdmin = await getFundedClient();

    console.log("Admin address:       ", admin.senderAddress());
    console.log("RecordAdmin address: ", recordAdmin.senderAddress());

    const { output: trail, response } = await createTrailWithSeedRecord(admin);

    console.log(`Created trail ${trail.id} with transaction ${response.digest}`);
    console.log("Immutable metadata:", trail.immutableMetadata);
    console.log("Updatable metadata:", trail.updatableMetadata);
    console.log("Locking config:", trail.lockingConfig);

    assert.equal(trail.sequenceNumber, 1n);
    assert.ok(trail.immutableMetadata);
    assert.equal(trail.immutableMetadata?.name, "Example Audit Trail");

    // Define a RecordAdmin role and issue the capability to recordAdmin's address.
    const role = admin.trail(trail.id).access().forRole("RecordAdmin");
    await role
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await role
        .issueCapability(new CapabilityIssueOptions(recordAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const onChain = await admin.trail(trail.id).get();
    const roleNames = onChain.roles.roles.map((r) => r.name);
    console.log("Roles:", roleNames);
    assert.ok(roleNames.includes("RecordAdmin"));
}
