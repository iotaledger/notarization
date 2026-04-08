// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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

/**
 * Demonstrates how to:
 * 1. Create records via a delegated RecordMaintenance role.
 * 2. Delete a single record by sequence number.
 * 3. Batch-delete remaining records.
 */
export async function deleteRecords(): Promise<void> {
    console.log("=== Audit Trail: Delete Records ===\n");

    const client = await getFundedClient();
    const { output: trail } = await client
        .createTrail()
        .withTrailMetadata("Delete Records Example", "Trail configured to demonstrate record deletions")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withNone(), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Seed record", "v0")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    const trailId = trail.id;
    const trailHandle = client.trail(trailId);

    // Create a role with delete permissions
    const role = trailHandle.access().forRole("RecordMaintenance");
    await role
        .create(new PermissionSet([Permission.AddRecord, Permission.DeleteRecord, Permission.DeleteAllRecords]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Add records
    const rec1 = await trailHandle
        .records()
        .add(Data.fromString("First record"), "v1")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    const rec2 = await trailHandle
        .records()
        .add(Data.fromString("Second record"), "v2")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log("Added records", rec1.output.sequenceNumber, "and", rec2.output.sequenceNumber);

    // Delete a single record
    const deleted = await trailHandle
        .records()
        .delete(rec1.output.sequenceNumber)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Deleted record", deleted.output.sequenceNumber);

    let count = await trailHandle.records().recordCount();
    console.log("Record count after single delete:", count);
    assert.equal(count, 2n); // seed + rec2

    // Batch-delete remaining
    const batchDeleted = await trailHandle
        .records()
        .deleteBatch(BigInt(10))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Batch deleted", batchDeleted.output, "records");

    count = await trailHandle.records().recordCount();
    assert.equal(count, 0n, "all records should be deleted after batch");
    console.log("Record count after batch delete:", count);
}
