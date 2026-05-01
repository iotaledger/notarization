// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the RecordMaintenance role.
 * - **Maintenance admin client**: Holds the RecordMaintenance capability. Adds records and then
 *   deletes them individually and in batch.
 *
 * Demonstrates how to:
 * 1. Create records using a delegated record-maintenance role.
 * 2. Delete a single record by sequence number.
 * 3. Delete the remaining records in one batch.
 */

import { CapabilityIssueOptions, Data, Permission, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function deleteRecords(): Promise<void> {
    console.log("=== Audit Trail: Delete Records ===\n");

    // Use a maintenance client to show deletes happening through a delegated capability.
    const adminClient = await getFundedClient();
    const maintenanceAdminClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    const recordMaintenanceRole = adminClient.trail(trailId).access().forRole("RecordMaintenance");
    await recordMaintenanceRole
        .create(new PermissionSet([Permission.AddRecord, Permission.DeleteRecord, Permission.DeleteAllRecords]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await recordMaintenanceRole
        .issueCapability(new CapabilityIssueOptions(maintenanceAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const maintenanceRecords = maintenanceAdminClient.trail(trailId).records();

    const firstAddedRecord = await maintenanceRecords
        .add(Data.fromString("Second record"), "event:received")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);
    const secondAddedRecord = await maintenanceRecords
        .add(Data.fromString("Third record"), "event:dispatched")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);

    console.log(
        "Trail has records at sequence numbers 0,",
        firstAddedRecord.output.sequenceNumber,
        ",",
        secondAddedRecord.output.sequenceNumber,
    );
    assert.equal(await maintenanceRecords.recordCount(), 3n);

    const deletedRecord = await maintenanceRecords
        .delete(firstAddedRecord.output.sequenceNumber)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);
    console.log("Deleted record", deletedRecord.output.sequenceNumber);

    let recordCount = await maintenanceRecords.recordCount();
    console.log("Record count after single delete:", recordCount);
    assert.equal(recordCount, 2n);
    await assert.rejects(
        () => maintenanceRecords.get(firstAddedRecord.output.sequenceNumber),
        "deleted record should no longer be readable",
    );

    // Batch delete skips locked records and returns the deleted sequence numbers.
    const deletedRemaining = await maintenanceRecords
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(maintenanceAdminClient);

    console.log("Batch deleted the remaining sequence numbers:", deletedRemaining.output);
    assert.deepEqual(Array.from(deletedRemaining.output), [
        0n,
        secondAddedRecord.output.sequenceNumber,
    ]);
    recordCount = await maintenanceRecords.recordCount();
    assert.equal(recordCount, 0n);
}
