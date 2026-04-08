// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { CapabilityIssueOptions, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Create an audit trail with immutable metadata, updatable metadata, and a seed record.
 * 2. Inspect the built-in Admin role.
 * 3. Define a RecordAdmin role and issue a capability for it.
 */
export async function createAuditTrail(): Promise<void> {
    console.log("Creating an audit trail");

    const client = await getFundedClient();
    const { output: trail, response } = await createTrailWithSeedRecord(client);

    console.log(`Created trail ${trail.id} with transaction ${response.digest}`);
    console.log("Immutable metadata:", trail.immutableMetadata);
    console.log("Updatable metadata:", trail.updatableMetadata);
    console.log("Locking config:", trail.lockingConfig);

    assert.equal(trail.sequenceNumber, 1n);
    assert.ok(trail.immutableMetadata);
    assert.equal(trail.immutableMetadata?.name, "Example Audit Trail");

    // Define a RecordAdmin role and issue a capability
    const role = client.trail(trail.id).access().forRole("RecordAdmin");
    await role
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const onChain = await client.trail(trail.id).get();
    const roleNames = onChain.roles.roles.map((r) => r.name);
    console.log("Roles:", roleNames);
    assert.ok(roleNames.includes("RecordAdmin"));
}
