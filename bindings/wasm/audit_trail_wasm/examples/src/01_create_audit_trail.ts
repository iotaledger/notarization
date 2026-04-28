// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and holds the built-in Admin capability that is
 *   automatically minted on creation.
 * - **Record admin client**: Receives a RecordAdmin capability bound to their address. Writes
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

    // `adminClient` creates the trail and holds the Admin capability.
    // `recordAdminClient` receives the delegated RecordAdmin capability.
    const adminClient = await getFundedClient();
    const recordAdminClient = await getFundedClient();

    console.log("Admin client address:        ", adminClient.senderAddress());
    console.log("Record admin client address: ", recordAdminClient.senderAddress());

    const { output: createdTrail, response } = await createTrailWithSeedRecord(adminClient);

    console.log(`Created trail ${createdTrail.id} with transaction ${response.digest}`);
    console.log("Immutable metadata:", createdTrail.immutableMetadata);
    console.log("Updatable metadata:", createdTrail.updatableMetadata);
    console.log("Locking config:", createdTrail.lockingConfig);

    assert.equal(createdTrail.sequenceNumber, 1n);
    assert.ok(createdTrail.immutableMetadata);
    assert.equal(createdTrail.immutableMetadata?.name, "Example Audit Trail");

    // Admin capability authorization is implicit: adminClient owns the built-in Admin capability.
    const recordAdminRole = adminClient.trail(createdTrail.id).access().forRole("RecordAdmin");
    await recordAdminRole
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await recordAdminRole
        .issueCapability(new CapabilityIssueOptions(recordAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const onChainTrail = await adminClient.trail(createdTrail.id).get();
    const roleNames = onChainTrail.roles.roles.map((r) => r.name);
    console.log("Roles:", roleNames);
    assert.ok(roleNames.includes("RecordAdmin"));
}
