// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createLocked } from "./01_create_locked";
import { createDynamic } from "./02_create_dynamic";
import {afterEach} from "mocha";

// Only verifies that no uncaught exceptions are thrown, including syntax errors etc.
describe("Test node examples", function() {
    afterEach(
        () => {
            console.log("\n----------------------------------------------------\n");
        }
    )
    it("Should create Locked Notarization", async () => {
        await createLocked();
    });
    it("Should create Dynamic Notarization", async () => {
        await createDynamic();
    });
});

