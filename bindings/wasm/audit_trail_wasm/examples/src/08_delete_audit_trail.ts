// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { CapabilityIssueOptions, Data, Permission, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Show that a non-empty trail cannot be deleted.
 * 2. Empty the trail with deleteBatch.
 * 3. Delete the trail once its records are gone.
 */
export async function deleteAuditTrail(): Promise<void> {
    console.log("=== Audit Trail: Delete Trail ===\n");

    const client = await getFundedClient();
    const { output: created } = await client
        .createTrail()
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = created.id;
    const trailHandle = client.trail(trailId);

    // Create a role with delete permissions
    const role = trailHandle.access().forRole("MaintenanceAdmin");
    await role
        .create(new PermissionSet([Permission.DeleteAllRecords, Permission.DeleteAuditTrail]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 1. Attempting to delete a non-empty trail should fail
    let deleteWhileNonEmptySucceeded = false;
    try {
        await trailHandle.deleteAuditTrail().withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
        deleteWhileNonEmptySucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(deleteWhileNonEmptySucceeded, false, "a trail must be empty before deletion");
    console.log("Deleting the non-empty trail failed as expected.\n");

    // 2. Batch-delete all records
    const deletedRecords = await trailHandle
        .records()
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Deleted", deletedRecords.output, "record(s) before trail removal.\n");

    const count = await trailHandle.records().recordCount();
    assert.equal(count, 0n, "trail should have no records after batch delete");

    // 3. Delete the now-empty trail
    const deletedTrail = await trailHandle
        .deleteAuditTrail()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Trail deleted:");
    console.log("  trail_id =", deletedTrail.output.trailId);
    console.log("  timestamp =", deletedTrail.output.timestamp);

    let getAfterDeleteSucceeded = false;
    try {
        await trailHandle.get();
        getAfterDeleteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(getAfterDeleteSucceeded, false, "deleted trail should no longer be readable");
}