// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { CapabilityIssueOptions, Data, LockingConfig, LockingWindow, PermissionSet, TimeLock } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Load the full on-chain trail object.
 * 2. Inspect metadata, roles, and locking configuration.
 * 3. Read records individually and through pagination.
 * 4. Query the record-count and lock-status helpers.
 */
export async function accessReadOnlyMethods(): Promise<void> {
    console.log("=== Audit Trail: Read-Only Inspection ===\n");

    const client = await getFundedClient();
    const { output: created } = await client
        .createTrail()
        .withTrailMetadata("Operations Trail", "Used to inspect read-only accessors")
        .withUpdatableMetadata("Status: Active")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withCountBased(BigInt(2)), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Initial record", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = created.id;
    const trailHandle = client.trail(trailId);

    // Create RecordAdmin role
    const role = trailHandle.access().forRole("RecordAdmin");
    await role.create(PermissionSet.recordAdminPermissions()).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Add a follow-up record
    await trailHandle
        .records()
        .add(Data.fromString("Follow-up record"), "event:updated")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Read the full on-chain trail
    const onChain = await trailHandle.get();
    console.log("Trail summary:");
    console.log("  id =", onChain.id);
    console.log("  creator =", onChain.creator);
    console.log("  created_at =", onChain.createdAt);
    console.log("  sequence_number =", onChain.sequenceNumber);
    console.log("  immutable_metadata =", onChain.immutableMetadata);
    console.log("  updatable_metadata =", onChain.updatableMetadata, "\n");

    console.log("Roles:", onChain.roles.roles.map((r) => r.name));
    console.log("Locking config:", onChain.lockingConfig, "\n");

    // Query helpers
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