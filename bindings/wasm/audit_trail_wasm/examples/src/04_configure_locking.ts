// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import {
    CapabilityIssueOptions,
    Data,
    LockingConfig,
    LockingWindow,
    PermissionSet,
    TimeLock,
} from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Delegate locking updates through a LockingAdmin role.
 * 2. Freeze record creation with a write lock.
 * 3. Restore writes and add a new record.
 * 4. Update the delete-record window and delete-trail lock.
 */
export async function configureLocking(): Promise<void> {
    console.log("=== Audit Trail: Configure Locking ===\n");

    const client = await getFundedClient();
    const { output: trail } = await createTrailWithSeedRecord(client);
    const trailId = trail.id;
    const trailHandle = client.trail(trailId);

    // Create LockingAdmin and RecordAdmin roles
    const lockingRole = trailHandle.access().forRole("LockingAdmin");
    await lockingRole
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await lockingRole
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const recordRole = trailHandle.access().forRole("RecordAdmin");
    await recordRole
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await recordRole
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Freeze writes
    await trailHandle
        .locking()
        .updateWriteLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const locked = await trailHandle.get();
    console.log("Write lock after update:", locked.lockingConfig.writeLock, "\n");
    assert.equal(locked.lockingConfig.writeLock.type, TimeLock.withInfinite().type);

    // Attempt to add a record while locked — should fail
    const blockedAdd = await trailHandle
        .records()
        .add(Data.fromString("This write should fail"), "blocked")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client)
        .catch(() => null);
    assert.equal(blockedAdd, null, "write lock should block adding records");

    // Lift the write lock
    await trailHandle
        .locking()
        .updateWriteLock(TimeLock.withNone())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const added = await trailHandle
        .records()
        .add(Data.fromString("Write lock lifted"), "event:resumed")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Added record", added.output.sequenceNumber, "after clearing the write lock.\n");

    // Configure deletion window and trail lock
    await trailHandle
        .locking()
        .updateDeleteRecordWindow(LockingWindow.withCountBased(BigInt(2)))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await trailHandle
        .locking()
        .updateDeleteTrailLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const finalState = await trailHandle.get();
    console.log("Final locking config:");
    console.log("  delete_record_window =", finalState.lockingConfig.deleteRecordWindow);
    console.log("  delete_trail_lock =", finalState.lockingConfig.deleteTrailLock);
    console.log("  write_lock =", finalState.lockingConfig.writeLock);

    assert.equal(finalState.lockingConfig.deleteRecordWindow.type, LockingWindow.withCountBased(BigInt(2)).type);
    assert.equal(finalState.lockingConfig.deleteTrailLock.type, TimeLock.withInfinite().type);
    assert.equal(finalState.lockingConfig.writeLock.type, TimeLock.withNone().type);
}
