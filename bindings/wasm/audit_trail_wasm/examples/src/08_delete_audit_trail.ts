// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the MaintenanceAdmin role.
 * - **Maintenance admin client**: Holds delete permissions. Attempts (and fails) to delete the
 *   non-empty trail, then batch-deletes all records before removing the trail itself.
 *
 * Demonstrates how to:
 * 1. Show that a non-empty trail cannot be deleted.
 * 2. Empty the trail with deleteBatch.
 * 3. Delete the trail once its records are gone.
 */

import { CapabilityIssueOptions, Data, Permission, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function deleteAuditTrail(): Promise<void> {
    console.log("=== Audit Trail: Delete Trail ===\n");

    // `adminClient` creates the trail and delegates trail maintenance.
    // `maintenanceAdminClient` empties and deletes the trail.
    const adminClient = await getFundedClient();
    const maintenanceAdminClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    const maintenanceAdminRole = adminClient.trail(trailId).access().forRole("MaintenanceAdmin");
    await maintenanceAdminRole
        .create(new PermissionSet([Permission.DeleteAllRecords, Permission.DeleteAuditTrail]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await maintenanceAdminRole
        .issueCapability(new CapabilityIssueOptions(maintenanceAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const maintenanceTrail = maintenanceAdminClient.trail(trailId);

    // 1. Attempting to delete a non-empty trail should fail.
    let deleteWhileNonEmptySucceeded = false;
    try {
        await maintenanceTrail
            .deleteAuditTrail()
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(maintenanceAdminClient);
        deleteWhileNonEmptySucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(deleteWhileNonEmptySucceeded, false, "a trail must be empty before deletion");
    console.log("Deleting the non-empty trail failed as expected.\n");

    // 2. Batch-delete all records.
    const deletedRecords = await maintenanceTrail
        .records()
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);
    console.log("Deleted", deletedRecords.output, "record(s) before trail removal.\n");

    const count = await maintenanceTrail.records().recordCount();
    assert.equal(count, 0n, "trail should have no records after batch delete");

    // 3. Delete the now-empty trail.
    const deletedTrail = await maintenanceTrail
        .deleteAuditTrail()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);
    console.log("Trail deleted:");
    console.log("  trail_id =", deletedTrail.output.trailId);
    console.log("  timestamp =", deletedTrail.output.timestamp);

    let getAfterDeleteSucceeded = false;
    try {
        await maintenanceTrail.get();
        getAfterDeleteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(getAfterDeleteSucceeded, false, "deleted trail should no longer be readable");
}
