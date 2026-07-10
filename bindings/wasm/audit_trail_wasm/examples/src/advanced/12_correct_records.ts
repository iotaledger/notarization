// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail, defines the RecordAdmin role, and issues a capability
 *   bound to `recordAdminClient`'s address.
 * - **Record admin client**: Holds the capability. Appends a correction record, resolves the
 *   current record, and verifies that the original record cannot be corrected again.
 *
 * Demonstrates how to:
 * 1. Append a correction record that supersedes an existing record.
 * 2. Read the original and correction records directly.
 * 3. Resolve the current record from the original sequence number.
 * 4. Show that an already replaced record cannot be corrected again.
 */

import { CapabilityIssueOptions, Data, PermissionSet } from "@iota/audit-trails/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "../util";

export async function correctRecords(): Promise<void> {
    console.log("=== Audit Trail Advanced: Correct Records ===\n");

    const adminClient = await getFundedClient();
    const recordAdminClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withInitialRecordString("Invoice total: 100 USD", "status:draft")
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

    const records = recordAdminClient.trail(trailId).records();

    const correction = await records
        .correct(0n, Data.fromString("Invoice total: 110 USD"), "status:corrected")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient);

    console.log("Corrected record 0 by appending record", correction.output.sequenceNumber, "\n");

    const original = await records.get(0n);
    const correctionRecord = await records.get(correction.output.sequenceNumber);
    const current = await records.resolveCurrent(0n);

    assert.equal(
        original.correction.isReplacedBy,
        correction.output.sequenceNumber,
        "the original record must point to the correction",
    );
    assert.ok(
        correctionRecord.correction.replaces.includes(0n),
        "the correction must reference the original sequence number",
    );
    assert.equal(
        current.sequenceNumber,
        correction.output.sequenceNumber,
        "resolveCurrent must return the appended correction",
    );
    assert.equal(current.data.toString(), "Invoice total: 110 USD");

    let secondCorrectionSucceeded = false;
    try {
        await records
            .correct(0n, Data.fromString("Invoice total: 120 USD"), "status:second-correction")
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(recordAdminClient);
        secondCorrectionSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(secondCorrectionSucceeded, false, "an already replaced record must not be corrected again");

    console.log("Original record:", original);
    console.log("Correction record:", correctionRecord);
    console.log("Current record resolved from #0:", current);
}
