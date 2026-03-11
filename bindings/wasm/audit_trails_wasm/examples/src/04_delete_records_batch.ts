// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function deleteRecordsBatch(): Promise<void> {
    console.log("Deleting records in batch");

    const client = await getFundedClient();
    const { output: trail } = await createTrailWithSeedRecord(client);
    const records = client.trail(trail.id).records();

    await records.addString("record 2", "second").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    await records.addString("record 3", "third").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);

    const before = await records.recordCount();
    const deleted = await records.deleteBatch(BigInt(2)).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    const after = await records.recordCount();

    console.log(`Deleted ${deleted.output} records. Count before=${before}, after=${after}`);

    assert.equal(before, 3);
    assert.equal(deleted.output, 2);
    assert.equal(after, 1);
}
