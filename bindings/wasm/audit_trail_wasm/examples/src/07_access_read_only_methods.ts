// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and sets up the RecordAdmin role.
 * - **RecordAdmin**: Adds one follow-up record. All subsequent operations are read-only
 *   and can be performed by any address — no capability required.
 *
 * Demonstrates how to:
 * 1. Load the full on-chain trail object.
 * 2. Inspect metadata, roles, and locking configuration.
 * 3. Read records individually and through pagination.
 * 4. Query the record-count and lock-status helpers.
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
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function accessReadOnlyMethods(): Promise<void> {
    console.log("=== Audit Trail: Read-Only Inspection ===\n");

    // `admin` creates the trail and sets up the role.
    // `recordAdmin` adds the follow-up record.
    const admin = await getFundedClient();
    const recordAdmin = await getFundedClient();

    const { output: created } = await admin
        .createTrail()
        .withTrailMetadata("Operations Trail", "Used to inspect read-only accessors")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withCountBased(BigInt(2)), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const trailId = created.id;

    // Create RecordAdmin role and issue to recordAdmin.
    const role = admin.trail(trailId).access().forRole("RecordAdmin");
    await role
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await role
        .issueCapability(new CapabilityIssueOptions(recordAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // RecordAdmin adds a follow-up record.
    await recordAdmin
        .trail(trailId)
        .records()
        .add(Data.fromString("Follow-up record"), "event:updated")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdmin);

    // All reads below require no capability — any address can inspect the trail.
    const onChain = await admin.trail(trailId).get();
    console.log("Trail summary:");
    console.log("  id =", onChain.id);
    console.log("  creator =", onChain.creator);
    console.log("  created_at =", onChain.createdAt);
    console.log("  sequence_number =", onChain.sequenceNumber);
    console.log("  immutable_metadata =", onChain.immutableMetadata);
    console.log("  updatable_metadata =", onChain.updatableMetadata, "\n");

    console.log("Roles:", onChain.roles.roles.map((r) => r.name));
    console.log("Locking config:", onChain.lockingConfig, "\n");

    const trailHandle = admin.trail(trailId);
    const count = await trailHandle.records().recordCount();
    const initialRecord = await trailHandle.records().get(0n);
    const firstPage = await trailHandle.records().listPage(undefined, 10);
    const recordZeroLocked = await trailHandle.locking().isRecordLocked(0n);

    console.log("Record count:", count);
    console.log("Record #0:", initialRecord);
    console.log("First page size:", firstPage.records.length, "(has_next_page =", firstPage.hasNextPage, ")");
    console.log("Is record #0 locked?", recordZeroLocked);

    assert.equal(count, 2n);
    assert.equal(initialRecord.data.toString(), "Initial record");
    assert.equal(firstPage.records.length, 2);
}
