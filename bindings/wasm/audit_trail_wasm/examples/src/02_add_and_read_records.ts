// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { Data } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, grantSelfRecordPermissions, TEST_GAS_BUDGET } from "./util";

/**
 * Demonstrates how to:
 * 1. Add follow-up records to a trail.
 * 2. Read them back individually by sequence number.
 * 3. Paginate through records.
 */
export async function addAndReadRecords(): Promise<void> {
    console.log("Adding records and reading them back with pagination");

    const client = await getFundedClient();
    const { output: trail } = await createTrailWithSeedRecord(client);
    await grantSelfRecordPermissions(client, trail.id);
    const records = client.trail(trail.id).records();

    // Add records
    const addedSecond = await records
        .add(Data.fromString("record 2"), "second")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    const addedThird = await records
        .add(Data.fromString("record 3"), "third")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

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