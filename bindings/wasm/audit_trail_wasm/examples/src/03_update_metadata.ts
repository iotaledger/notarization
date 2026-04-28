// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up the MetadataAdmin role.
 * - **Metadata admin client**: Holds the MetadataAdmin capability and updates the trail's mutable
 *   status field. Has no record-write permissions.
 *
 * Demonstrates how to:
 * 1. Create a trail with immutable and updatable metadata.
 * 2. Delegate metadata updates through a dedicated MetadataAdmin role.
 * 3. Change and clear the trail's updatable metadata.
 * 4. Verify that immutable metadata never changes.
 */

import { CapabilityIssueOptions, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function updateMetadata(): Promise<void> {
    console.log("=== Audit Trail: Update Metadata ===\n");

    // `adminClient` creates the trail and delegates metadata updates.
    // `metadataAdminClient` holds the MetadataAdmin capability and updates the status.
    const adminClient = await getFundedClient();
    const metadataAdminClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withTrailMetadata("Shipment Processing", "Tracks the lifecycle of a warehouse shipment")
        .withUpdatableMetadata("Status: Draft")
        .withInitialRecordString("Shipment created", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    // Delegate metadata updates to a MetadataAdmin role.
    const metadataAdminRole = adminClient.trail(trailId).access().forRole("MetadataAdmin");
    await metadataAdminRole
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await metadataAdminRole
        .issueCapability(new CapabilityIssueOptions(metadataAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailBeforeUpdate = await adminClient.trail(trailId).get();
    console.log("Before update:");
    console.log("  immutable =", trailBeforeUpdate.immutableMetadata);
    console.log("  updatable =", trailBeforeUpdate.updatableMetadata, "\n");

    // MetadataAdmin updates the mutable metadata.
    await metadataAdminClient
        .trail(trailId)
        .updateMetadata("Status: In Review")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(metadataAdminClient);

    const trailAfterUpdate = await adminClient.trail(trailId).get();
    console.log("After update:");
    console.log("  immutable =", trailAfterUpdate.immutableMetadata);
    console.log("  updatable =", trailAfterUpdate.updatableMetadata, "\n");

    assert.equal(trailAfterUpdate.immutableMetadata?.name, "Shipment Processing");
    assert.equal(trailAfterUpdate.updatableMetadata, "Status: In Review");

    // MetadataAdmin clears the mutable metadata.
    await metadataAdminClient
        .trail(trailId)
        .updateMetadata(undefined)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(metadataAdminClient);

    const trailAfterClear = await adminClient.trail(trailId).get();
    console.log("After clear:");
    console.log("  immutable =", trailAfterClear.immutableMetadata);
    console.log("  updatable =", trailAfterClear.updatableMetadata);

    assert.equal(trailAfterClear.immutableMetadata?.name, "Shipment Processing");
    assert.equal(trailAfterClear.updatableMetadata, undefined);
}
