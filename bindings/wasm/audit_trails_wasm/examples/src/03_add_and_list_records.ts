// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { Data } from "@iota/audit-trails/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, grantSelfRecordPermissions, TEST_GAS_BUDGET } from "./util";

export async function addAndListRecords(): Promise<void> {
    console.log("Adding records and reading them back with pagination");

    const client = await getFundedClient();
    const { output: trail } = await createTrailWithSeedRecord(client);
    await grantSelfRecordPermissions(client, trail.id);
    const records = client.trail(trail.id).records();

    const addedString = await records
        .add(Data.fromString("record 2"), "second")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    const addedThird = await records
        .add(Data.fromString("record 3"), "third")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log("Added records:", addedString.output, addedThird.output);

    const allRecords = await records.list();
    const firstPage = await records.listPage(undefined, 2);
    const secondPage = await records.listPage(firstPage.nextCursor, 2);

    console.log("All records:", allRecords);
    console.log("First page:", firstPage);
    console.log("Second page:", secondPage);

    assert.equal(allRecords.length, 3);
    assert.equal(firstPage.records.length, 2);
    assert.equal(firstPage.hasNextPage, true);
    assert.equal(secondPage.records.length, 1);
}
