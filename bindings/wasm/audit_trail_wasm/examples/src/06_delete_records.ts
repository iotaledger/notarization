// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and sets up the RecordMaintenance role.
 * - **RecordMaintainer**: Holds the RecordMaintenance capability. Adds records and then
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

    // `admin` creates the trail and sets up the role.
    // `recordMaintainer` adds and deletes records.
    const admin = await getFundedClient();
    const recordMaintainer = await getFundedClient();

    const { output: trail } = await admin
        .createTrail()
        .withTrailMetadata("Delete Records Example", "Trail configured to demonstrate record deletions")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withNone(), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Seed record", "v0")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const trailId = trail.id;

    // Create a role with delete permissions and issue to recordMaintainer.
    const role = admin.trail(trailId).access().forRole("RecordMaintenance");
    await role
        .create(new PermissionSet([Permission.AddRecord, Permission.DeleteRecord, Permission.DeleteAllRecords]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await role
        .issueCapability(new CapabilityIssueOptions(recordMaintainer.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const records = recordMaintainer.trail(trailId).records();

    // RecordMaintainer adds records.
    const rec1 = await records
        .add(Data.fromString("First record"), "v1")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainer);
    const rec2 = await records
        .add(Data.fromString("Second record"), "v2")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainer);

    console.log("Added records", rec1.output.sequenceNumber, "and", rec2.output.sequenceNumber);

    // Delete a single record.
    const deleted = await records
        .delete(rec1.output.sequenceNumber)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainer);
    console.log("Deleted record", deleted.output.sequenceNumber);

    let count = await records.recordCount();
    console.log("Record count after single delete:", count);
    assert.equal(count, 2n); // seed + rec2

    // Batch-delete remaining records.
    const batchDeleted = await records
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordMaintainer);
    console.log("Batch deleted", batchDeleted.output, "records");

    count = await records.recordCount();
    assert.equal(count, 0n, "all records should be deleted after batch");
    console.log("Record count after batch delete:", count);
}
