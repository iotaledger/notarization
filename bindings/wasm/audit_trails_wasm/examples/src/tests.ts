// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, it } from "mocha";

import { createTrail } from "./01_create_trail";
import { fetchTrail } from "./02_fetch_trail";
import { addAndListRecords } from "./03_add_and_list_records";
import { deleteRecordsBatch } from "./04_delete_records_batch";

describe("Audit trail wasm node examples", function() {
    afterEach(() => {
        console.log("\n----------------------------------------------------\n");
    });

    it("creates a trail", async () => {
        await createTrail();
    });

    it("fetches a trail", async () => {
        await fetchTrail();
    });

    it("adds and lists records", async () => {
        await addAndListRecords();
    });

    it("deletes records in batch", async () => {
        await deleteRecordsBatch();
    });
});
