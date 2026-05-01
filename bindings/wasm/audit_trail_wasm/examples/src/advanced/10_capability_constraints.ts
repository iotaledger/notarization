// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail, defines the RecordAdmin role, and issues a capability
 *   bound specifically to `intendedWriterClient`'s address. Also performs revocation.
 * - **Intended writer client**: The authorised holder. Writes a record successfully before
 *   revocation, then is blocked after the capability is revoked.
 * - **Wrong writer client**: An unauthorised actor who attempts to use the address-bound capability.
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

    const adminClient = await getFundedClient();
    const intendedWriterClient = await getFundedClient();
    const wrongWriterClient = await getFundedClient();

    const { output: createdTrail } = await createTrailWithSeedRecord(adminClient);
    const trailId = createdTrail.id;

    // Create the role before delegating the address-bound capability.
    await adminClient
        .trail(trailId)
        .access()
        .forRole("RecordAdmin")
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const recordAdminCapability = await adminClient
        .trail(trailId)
        .access()
        .forRole("RecordAdmin")
        .issueCapability(new CapabilityIssueOptions(intendedWriterClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    console.log(
        "Issued capability",
        recordAdminCapability.output.capabilityId,
        "to",
        intendedWriterClient.senderAddress(),
        "\n",
    );

    // The wrong wallet should not be able to add a record.
    let wrongWriterSucceeded = false;
    try {
        await wrongWriterClient
            .trail(trailId)
            .records()
            .add(Data.fromString("Wrong writer"), undefined, undefined)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(wrongWriterClient);
        wrongWriterSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(wrongWriterSucceeded, false, "a capability bound to another address must not be usable");

    // The intended writer CAN add a record.
    const authorizedRecord = await intendedWriterClient
        .trail(trailId)
        .records()
        .add(Data.fromString("Authorized writer"), undefined, undefined)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(intendedWriterClient);

    console.log("Bound holder added record", authorizedRecord.output.sequenceNumber, "successfully.\n");

    // Revoke the capability.
    await adminClient
        .trail(trailId)
        .access()
        .revokeCapability(recordAdminCapability.output.capabilityId, recordAdminCapability.output.validUntil)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // The intended writer should no longer be able to add a record.
    let revokedSucceeded = false;
    try {
        await intendedWriterClient
            .trail(trailId)
            .records()
            .add(Data.fromString("Should fail after revoke"), undefined, undefined)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(intendedWriterClient);
        revokedSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(revokedSucceeded, false, "revoked capabilities must no longer authorize record writes");

    console.log(
        "Revoked capability",
        recordAdminCapability.output.capabilityId,
        "and verified it can no longer be used.",
    );
}
