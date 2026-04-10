// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail, defines the RecordAdmin role, and issues a capability.
 * - **RecordAdmin**: Holds the capability and writes records. Reads are also done through
 *   this client to demonstrate that any address can read, but only the cap holder can write.
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

    // `admin` creates the trail and sets up the role.
    // `recordAdmin` holds the capability and writes/reads records.
    const admin = await getFundedClient();
    const recordAdmin = await getFundedClient();

    const { output: trail } = await createTrailWithSeedRecord(admin);
    const trailId = trail.id;

    // Create a RecordAdmin role and issue the capability to recordAdmin's address.
    const role = admin.trail(trailId).access().forRole("RecordAdmin");
    await role
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await role
        .issueCapability(new CapabilityIssueOptions(recordAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // The client automatically finds the capability in recordAdmin's wallet.
    const records = recordAdmin.trail(trailId).records();

    // Add records
    const addedSecond = await records
        .add(Data.fromString("record 2"), "second")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdmin);
    const addedThird = await records
        .add(Data.fromString("record 3"), "third")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(recordAdmin);

    console.log("Added records:", addedSecond.output, addedThird.output);

    // Read individual records
    const initial = await records.get(0n);
    const first = await records.get(addedSecond.output.sequenceNumber);
    assert.equal(initial.data.toString(), "seed record");
    assert.equal(first.data.toString(), "record 2");

    // Paginate
    const firstPage = await records.listPage(undefined, 2);
    const secondPage = await records.listPage(firstPage.nextCursor, 2);

    console.log("First page:", firstPage);
    console.log("Second page:", secondPage);

    assert.equal(firstPage.records.length, 2);
    assert.equal(firstPage.hasNextPage, true);
    assert.equal(secondPage.records.length, 1);
}
