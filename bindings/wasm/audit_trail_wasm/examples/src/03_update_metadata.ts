// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and sets up the MetadataAdmin role.
 * - **MetadataAdmin**: Holds the MetadataAdmin capability and updates the trail's mutable
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

    // `admin` creates the trail and sets up the role.
    // `metadataAdmin` holds the MetadataAdmin capability and updates the status.
    const admin = await getFundedClient();
    const metadataAdmin = await getFundedClient();

    const { output: trail } = await admin
        .createTrail()
        .withTrailMetadata("Shipment Processing", "Tracks the lifecycle of a warehouse shipment")
        .withUpdatableMetadata("Status: Draft")
        .withInitialRecordString("Shipment created", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const trailId = trail.id;

    // Delegate metadata updates to a MetadataAdmin role.
    const role = admin.trail(trailId).access().forRole("MetadataAdmin");
    await role
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await role
        .issueCapability(new CapabilityIssueOptions(metadataAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const before = await admin.trail(trailId).get();
    console.log("Before update:");
    console.log("  immutable =", before.immutableMetadata);
    console.log("  updatable =", before.updatableMetadata, "\n");

    // MetadataAdmin updates the mutable metadata.
    await metadataAdmin
        .trail(trailId)
        .updateMetadata("Status: In Review")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(metadataAdmin);

    const afterUpdate = await admin.trail(trailId).get();
    console.log("After update:");
    console.log("  immutable =", afterUpdate.immutableMetadata);
    console.log("  updatable =", afterUpdate.updatableMetadata, "\n");

    assert.equal(afterUpdate.immutableMetadata?.name, "Shipment Processing");
    assert.equal(afterUpdate.updatableMetadata, "Status: In Review");

    // MetadataAdmin clears the mutable metadata.
    await metadataAdmin
        .trail(trailId)
        .updateMetadata(undefined)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(metadataAdmin);

    const afterClear = await admin.trail(trailId).get();
    console.log("After clear:");
    console.log("  immutable =", afterClear.immutableMetadata);
    console.log("  updatable =", afterClear.updatableMetadata);

    assert.equal(afterClear.immutableMetadata?.name, "Shipment Processing");
    assert.equal(afterClear.updatableMetadata, undefined);
}
