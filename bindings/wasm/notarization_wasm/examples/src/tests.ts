// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createLocked } from "./0_create_locked";
import { createDynamic } from "./1_create_dynamic";

// Only verifies that no uncaught exceptions are thrown, including syntax errors etc.
describe("Test node examples", function() {
    it("Should create Locked Notarization", async () => {
        await createLocked();
    });
    it("Should create Dynamic Notarization", async () => {
        await createDynamic();
    });
});

