// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail, defines the RecordAdmin role, and issues a capability
 *   bound specifically to `intendedWriter`'s address. Also performs revocation.
 * - **IntendedWriter**: The authorised holder. Writes a record successfully before
 *   revocation, then is blocked after the capability is revoked.
 * - **WrongWriter**: An unauthorised actor who attempts to use the address-bound capability.
 *   All write attempts are rejected by the Move contract.
 *
 * Demonstrates how to:
 * 1. Bind a capability to a specific wallet address.
 * 2. Show that a different wallet cannot use it.
 * 3. Revoke the capability and confirm the bound holder can no longer use it.
 */

import { CapabilityIssueOptions, Data, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "../util";

export async function capabilityConstraints(): Promise<void> {
    console.log("=== Audit Trail Advanced: Capability Constraints ===\n");

    const admin = await getFundedClient();
    const intendedWriter = await getFundedClient();
    const wrongWriter = await getFundedClient();

    const { output: created } = await createTrailWithSeedRecord(admin);
    const trailId = created.id;

    // Create a RecordAdmin role.
    await admin
        .trail(trailId)
        .access()
        .forRole("RecordAdmin")
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // Issue a capability bound to the intended writer's address.
    const issued = await admin
        .trail(trailId)
        .access()
        .forRole("RecordAdmin")
        .issueCapability(new CapabilityIssueOptions(intendedWriter.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    console.log("Issued capability", issued.output.capabilityId, "to", intendedWriter.senderAddress(), "\n");

    // The wrong wallet should not be able to add a record.
    let wrongWriterSucceeded = false;
    try {
        await wrongWriter
            .trail(trailId)
            .records()
            .add(Data.fromString("Wrong writer"), undefined, undefined)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(wrongWriter);
        wrongWriterSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(wrongWriterSucceeded, false, "a capability bound to another address must not be usable");

    // The intended writer CAN add a record.
    const added = await intendedWriter
        .trail(trailId)
        .records()
        .add(Data.fromString("Authorized writer"), undefined, undefined)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(intendedWriter);

    console.log("Bound holder added record", added.output.sequenceNumber, "successfully.\n");

    // Revoke the capability.
    await admin
        .trail(trailId)
        .access()
        .revokeCapability(issued.output.capabilityId, issued.output.validUntil)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // The intended writer should no longer be able to add a record.
    let revokedSucceeded = false;
    try {
        await intendedWriter
            .trail(trailId)
            .records()
            .add(Data.fromString("Should fail after revoke"), undefined, undefined)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(intendedWriter);
        revokedSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(revokedSucceeded, false, "revoked capabilities must no longer authorize record writes");

    console.log("Revoked capability", issued.output.capabilityId, "and verified it can no longer be used.");
}
