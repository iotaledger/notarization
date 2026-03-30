// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { strict as assert } from "assert";
import { Data } from "@iota/audit-trail/node";
import { createTrailWithSeedRecord, getFundedClient, grantSelfRecordPermissions, TEST_GAS_BUDGET } from "./util";

export async function deleteRecordsBatch(): Promise<void> {
    console.log("Deleting records in batch");

    const client = await getFundedClient();
    const { output: trail } = await createTrailWithSeedRecord(client);
    await grantSelfRecordPermissions(client, trail.id);
    const records = client.trail(trail.id).records();

    await records.add(Data.fromString("record 2"), "second").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    await records.add(Data.fromString("record 3"), "third").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);

    const before = await records.recordCount();
    const deleted = await records.deleteBatch(BigInt(2)).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    const after = await records.recordCount();

    console.log(`Deleted ${deleted.output} records. Count before=${before}, after=${after}`);

    assert.equal(before, 3n);
    assert.equal(deleted.output, 2n);
    assert.equal(after, 1n);
}
