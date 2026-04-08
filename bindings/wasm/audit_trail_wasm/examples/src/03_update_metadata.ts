// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { CapabilityIssueOptions, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Create a trail with immutable and updatable metadata.
 * 2. Delegate metadata updates through a dedicated MetadataAdmin role.
 * 3. Change and clear the trail's updatable metadata.
 * 4. Verify that immutable metadata never changes.
 */
export async function updateMetadata(): Promise<void> {
    console.log("=== Audit Trail: Update Metadata ===\n");

    const client = await getFundedClient();
    const { output: trail } = await client
        .createTrail()
        .withTrailMetadata("Shipment Processing", "Tracks the lifecycle of a warehouse shipment")
        .withUpdatableMetadata("Status: Draft")
        .withInitialRecordString("Shipment created", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = trail.id;
    const trailHandle = client.trail(trailId);

    // Delegate metadata updates to a MetadataAdmin role
    const role = trailHandle.access().forRole("MetadataAdmin");
    await role.create(PermissionSet.metadataAdminPermissions()).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const before = await trailHandle.get();
    console.log("Before update:");
    console.log("  immutable =", before.immutableMetadata);
    console.log("  updatable =", before.updatableMetadata, "\n");

    // Update the mutable metadata
    await trailHandle
        .updateMetadata("Status: In Review")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const afterUpdate = await trailHandle.get();
    console.log("After update:");
    console.log("  immutable =", afterUpdate.immutableMetadata);
    console.log("  updatable =", afterUpdate.updatableMetadata, "\n");

    assert.equal(afterUpdate.immutableMetadata?.name, "Shipment Processing");
    assert.equal(afterUpdate.updatableMetadata, "Status: In Review");

    // Clear the mutable metadata
    await trailHandle.updateMetadata(undefined).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);

    const afterClear = await trailHandle.get();
    console.log("After clear:");
    console.log("  immutable =", afterClear.immutableMetadata);
    console.log("  updatable =", afterClear.updatableMetadata);

    assert.equal(afterClear.immutableMetadata?.name, "Shipment Processing");
    assert.equal(afterClear.updatableMetadata, undefined);
}