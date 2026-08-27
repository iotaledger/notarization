// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, it } from "node:test";

import { examples } from "./examples.js";

describe("Proof of Inclusion wasm node examples", () => {
    afterEach(() => {
        console.log("\n----------------------------------------------------\n");
    });

    for (const example of Object.values(examples)) {
        it(example.testName, example.run);
    }
});
