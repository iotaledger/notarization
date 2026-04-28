// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the RecordMaintenance role.
 * - **Record maintainer client**: Holds the RecordMaintenance capability. Adds records and then
 *   deletes them individually and in batch.
 *
 * Demonstrates how to:
 * 1. Create records via a delegated RecordMaintenance role.
 * 2. Delete a single record by sequence number.
 * 3. Batch-delete remaining records.
 */

import {
    CapabilityIssueOptions,
    Data,
    LockingConfig,
    LockingWindow,
    Permission,
    PermissionSet,
    TimeLock,
} from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function deleteRecords(): Promise<void> {
    console.log("=== Audit Trail: Delete Records ===\n");

    // `adminClient` creates the trail and delegates record maintenance.
    // `recordMaintainerClient` adds and deletes records.
    const adminClient = await getFundedClient();
    const recordMaintainerClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withTrailMetadata("Delete Records Example", "Trail configured to demonstrate record deletions")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withNone(), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Seed record", "v0")
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
        .issueCapability(new CapabilityIssueOptions(recordMaintainerClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const maintenanceRecords = recordMaintainerClient.trail(trailId).records();

    // RecordMaintainer adds records.
    const firstMaintainedRecord = await maintenanceRecords
        .add(Data.fromString("First record"), "v1")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainerClient);
    const secondMaintainedRecord = await maintenanceRecords
        .add(Data.fromString("Second record"), "v2")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainerClient);

    console.log(
        "Added records",
        firstMaintainedRecord.output.sequenceNumber,
        "and",
        secondMaintainedRecord.output.sequenceNumber,
    );

    // Delete a single record.
    const deletedRecord = await maintenanceRecords
        .delete(firstMaintainedRecord.output.sequenceNumber)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainerClient);
    console.log("Deleted record", deletedRecord.output.sequenceNumber);

    let recordCount = await maintenanceRecords.recordCount();
    console.log("Record count after single delete:", recordCount);
    assert.equal(recordCount, 2n); // seed + secondMaintainedRecord

    // Batch-delete remaining records.
    const batchDeletedRecords = await maintenanceRecords
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainerClient);
    console.log("Batch deleted", batchDeletedRecords.output, "records");

    recordCount = await maintenanceRecords.recordCount();
    assert.equal(recordCount, 0n, "all records should be deleted after batch");
    console.log("Record count after batch delete:", recordCount);
}
