// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail, defines the RecordAdmin role, and issues a capability.
 * - **Record admin client**: Holds the capability and writes records. Reads use the same client
 *   to keep the example focused after delegation.
 *
 * Demonstrates how to:
 * 1. Add follow-up records to a trail.
 * 2. Read them back individually by sequence number.
 * 3. Paginate through records.
 */

import { CapabilityIssueOptions, Data, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function addAndReadRecords(): Promise<void> {
    console.log("Adding records and reading them back with pagination");

    // `adminClient` creates the trail and delegates record writes.
    // `recordAdminClient` holds the capability and writes/reads records.
    const adminClient = await getFundedClient();
    const recordAdminClient = await getFundedClient();

    const { output: createdTrail } = await createTrailWithSeedRecord(adminClient);
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

    // Capability selection is automatic from recordAdminClient's wallet.
    const recordAdminRecords = recordAdminClient.trail(trailId).records();

    const addedSecondRecord = await recordAdminRecords
        .add(Data.fromString("record 2"), "second")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient);
    const addedThirdRecord = await recordAdminRecords
        .add(Data.fromString("record 3"), "third")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdminClient);

    console.log("Added records:", addedSecondRecord.output, addedThirdRecord.output);

    const seedRecord = await recordAdminRecords.get(0n);
    const secondRecord = await recordAdminRecords.get(addedSecondRecord.output.sequenceNumber);
    assert.equal(seedRecord.data.toString(), "seed record");
    assert.equal(secondRecord.data.toString(), "record 2");

    // Pagination uses the previous page cursor to continue from the next record.
    const firstPage = await recordAdminRecords.listPage(undefined, 2);
    const secondPage = await recordAdminRecords.listPage(firstPage.nextCursor, 2);

    console.log("First page:", firstPage);
    console.log("Second page:", secondPage);

    assert.equal(firstPage.records.length, 2);
    assert.equal(firstPage.hasNextPage, true);
    assert.equal(secondPage.records.length, 1);
}
