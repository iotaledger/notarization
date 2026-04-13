// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and sets up the LockingAdmin and RecordAdmin roles.
 * - **LockingAdmin**: Controls write and delete locks. Holds the LockingAdmin capability.
 * - **RecordAdmin**: Writes records. Used to demonstrate that the write lock is enforced
 *   per-sender, not just checked by the admin.
 *
 * Demonstrates how to:
 * 1. Delegate locking updates through a LockingAdmin role.
 * 2. Freeze record creation with a write lock.
 * 3. Restore writes and add a new record.
 * 4. Update the delete-record window and delete-trail lock.
 */

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

export async function configureLocking(): Promise<void> {
    console.log("=== Audit Trail: Configure Locking ===\n");

    // `admin` creates the trail and sets up roles.
    // `lockingAdmin` controls locks; `recordAdmin` writes records.
    const admin = await getFundedClient();
    const lockingAdmin = await getFundedClient();
    const recordAdmin = await getFundedClient();

    const { output: trail } = await createTrailWithSeedRecord(admin);
    const trailId = trail.id;

    // Create LockingAdmin and RecordAdmin roles.
    const lockingRole = admin.trail(trailId).access().forRole("LockingAdmin");
    await lockingRole
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await lockingRole
        .issueCapability(new CapabilityIssueOptions(lockingAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const recordRole = admin.trail(trailId).access().forRole("RecordAdmin");
    await recordRole
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await recordRole
        .issueCapability(new CapabilityIssueOptions(recordAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // LockingAdmin freezes writes.
    await lockingAdmin
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdmin);

    const locked = await admin.trail(trailId).get();
    console.log("Write lock after update:", locked.lockingConfig.writeLock, "\n");
    assert.equal(locked.lockingConfig.writeLock.type, TimeLock.withInfinite().type);

    // RecordAdmin attempts to add a record while locked — should fail.
    const blockedAdd = await recordAdmin
        .trail(trailId)
        .records()
        .add(Data.fromString("This write should fail"), "blocked")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdmin)
        .catch(() => null);
    assert.equal(blockedAdd, null, "write lock should block adding records");

    // LockingAdmin lifts the write lock.
    await lockingAdmin
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withNone())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdmin);

    const added = await recordAdmin
        .trail(trailId)
        .records()
        .add(Data.fromString("Write lock lifted"), "event:resumed")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdmin);
    console.log("Added record", added.output.sequenceNumber, "after clearing the write lock.\n");

    // LockingAdmin configures deletion window and trail lock.
    await lockingAdmin
        .trail(trailId)
        .locking()
        .updateDeleteRecordWindow(LockingWindow.withCountBased(BigInt(2)))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdmin);
    await lockingAdmin
        .trail(trailId)
        .locking()
        .updateDeleteTrailLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdmin);

    const finalState = await admin.trail(trailId).get();
    console.log("Final locking config:");
    console.log("  delete_record_window =", finalState.lockingConfig.deleteRecordWindow);
    console.log("  delete_trail_lock =", finalState.lockingConfig.deleteTrailLock);
    console.log("  write_lock =", finalState.lockingConfig.writeLock);

    assert.equal(finalState.lockingConfig.deleteRecordWindow.type, LockingWindow.withCountBased(BigInt(2)).type);
    assert.equal(finalState.lockingConfig.deleteTrailLock.type, TimeLock.withInfinite().type);
    assert.equal(finalState.lockingConfig.writeLock.type, TimeLock.withNone().type);
}
