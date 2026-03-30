// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createTrail } from "./01_create_trail";
import { fetchTrail } from "./02_fetch_trail";
import { addAndListRecords } from "./03_add_and_list_records";
import { deleteRecordsBatch } from "./04_delete_records_batch";

export async function main(example?: string) {
    const argument = example ?? process.argv?.[2]?.toLowerCase();
    if (!argument) {
        throw new Error("Please specify an example name, e.g. '01_create_trail'");
    }

    switch (argument) {
        case "01_create_trail":
            return createTrail();
        case "02_fetch_trail":
            return fetchTrail();
        case "03_add_and_list_records":
            return addAndListRecords();
        case "04_delete_records_batch":
            return deleteRecordsBatch();
        default:
            throw new Error(`Unknown example name: '${argument}'`);
    }
}

main().catch((error) => {
    console.error("Example error:", error);
});
