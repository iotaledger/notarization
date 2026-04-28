// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the RecordAdmin role.
 * - **Record admin client**: Adds one follow-up record. All subsequent operations are read-only
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

    // `adminClient` creates the trail and delegates one record write.
    // `recordAdminClient` adds the follow-up record.
    const adminClient = await getFundedClient();
    const recordAdminClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withTrailMetadata("Operations Trail", "Used to inspect read-only accessors")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withCountBased(BigInt(2)), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    const recordAdminRole = adminClient.trail(trailId).access().forRole("RecordAdmin");
    await recordAdminRole
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await recordAdminRole
        .issueCapability(new CapabilityIssueOptions(recordAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // RecordAdmin adds a follow-up record.
    await recordAdminClient
        .trail(trailId)
        .records()
        .add(Data.fromString("Follow-up record"), "event:updated")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient);

    // All reads below require no capability — any address can inspect the trail.
    const onChainTrail = await adminClient.trail(trailId).get();
    console.log("Trail summary:");
    console.log("  id =", onChainTrail.id);
    console.log("  creator =", onChainTrail.creator);
    console.log("  created_at =", onChainTrail.createdAt);
    console.log("  sequence_number =", onChainTrail.sequenceNumber);
    console.log("  immutable_metadata =", onChainTrail.immutableMetadata);
    console.log("  updatable_metadata =", onChainTrail.updatableMetadata, "\n");

    console.log("Roles:", onChainTrail.roles.roles.map((r) => r.name));
    console.log("Locking config:", onChainTrail.lockingConfig, "\n");

    const trailHandle = adminClient.trail(trailId);
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
