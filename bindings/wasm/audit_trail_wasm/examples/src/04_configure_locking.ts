// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the LockingAdmin and RecordAdmin roles.
 * - **Locking admin client**: Controls write and delete locks. Holds the LockingAdmin capability.
 * - **Record admin client**: Writes records. Used to demonstrate that the write lock is enforced
 *   per-sender, not just checked by the admin client.
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

    // `adminClient` creates the trail and delegates separate lock/write authority.
    // `lockingAdminClient` controls locks; `recordAdminClient` writes records.
    const adminClient = await getFundedClient();
    const lockingAdminClient = await getFundedClient();
    const recordAdminClient = await getFundedClient();

    const { output: createdTrail } = await createTrailWithSeedRecord(adminClient);
    const trailId = createdTrail.id;

    const lockingAdminRole = adminClient.trail(trailId).access().forRole("LockingAdmin");
    await lockingAdminRole
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await lockingAdminRole
        .issueCapability(new CapabilityIssueOptions(lockingAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const recordAdminRole = adminClient.trail(trailId).access().forRole("RecordAdmin");
    await recordAdminRole
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await recordAdminRole
        .issueCapability(new CapabilityIssueOptions(recordAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // LockingAdmin freezes writes.
    await lockingAdminClient
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdminClient);

    const lockedTrail = await adminClient.trail(trailId).get();
    console.log("Write lock after update:", lockedTrail.lockingConfig.writeLock, "\n");
    assert.equal(lockedTrail.lockingConfig.writeLock.type, TimeLock.withInfinite().type);

    // RecordAdmin attempts to add a record while locked — should fail.
    const blockedAdd = await recordAdminClient
        .trail(trailId)
        .records()
        .add(Data.fromString("This write should fail"), "blocked")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient)
        .catch(() => null);
    assert.equal(blockedAdd, null, "write lock should block adding records");

    // LockingAdmin lifts the write lock.
    await lockingAdminClient
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withNone())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdminClient);

    const recordAddedAfterUnlock = await recordAdminClient
        .trail(trailId)
        .records()
        .add(Data.fromString("Write lock lifted"), "event:resumed")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient);
    console.log("Added record", recordAddedAfterUnlock.output.sequenceNumber, "after clearing the write lock.\n");

    // LockingAdmin configures deletion window and trail lock.
    await lockingAdminClient
        .trail(trailId)
        .locking()
        .updateDeleteRecordWindow(LockingWindow.withCountBased(BigInt(2)))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdminClient);
    await lockingAdminClient
        .trail(trailId)
        .locking()
        .updateDeleteTrailLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdminClient);

    const finalTrail = await adminClient.trail(trailId).get();
    console.log("Final locking config:");
    console.log("  delete_record_window =", finalTrail.lockingConfig.deleteRecordWindow);
    console.log("  delete_trail_lock =", finalTrail.lockingConfig.deleteTrailLock);
    console.log("  write_lock =", finalTrail.lockingConfig.writeLock);

    assert.equal(finalTrail.lockingConfig.deleteRecordWindow.type, LockingWindow.withCountBased(BigInt(2)).type);
    assert.equal(finalTrail.lockingConfig.deleteTrailLock.type, TimeLock.withInfinite().type);
    assert.equal(finalTrail.lockingConfig.writeLock.type, TimeLock.withNone().type);
}
