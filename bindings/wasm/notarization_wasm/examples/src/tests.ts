// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createLocked } from "./01_create_locked";
import { createDynamic } from "./02_create_dynamic";
import { updateDynamic } from "./03_update_dynamic";
import { destroyNotarization } from "./04_destroy_notarization";
import { updateState } from "./05_update_state";
import { updateMetadata } from "./06_update_metadata";
import { transferNotarization } from "./07_transfer_notarization";
import { accessReadOnlyMethods } from "./08_access_read_only_methods";

import { afterEach } from "mocha";

// Only verifies that no uncaught exceptions are thrown, including syntax errors etc.
describe("Test node examples", function() {
    afterEach(
        () => {
            console.log("\n----------------------------------------------------\n");
        },
    );
    it("Should create Locked Notarization", async () => {
        await createLocked();
    });
    it("Should create Dynamic Notarization", async () => {
        await createDynamic();
    });
    it("Should update Dynamic Notarization", async () => {
        await updateDynamic();
    });
    it("Should destroy a Notarization", async () => {
        await destroyNotarization();
    });
    it("Should update state", async () => {
        await updateState();
    });
    it("Should update metadata", async () => {
        await updateMetadata();
    });
    it("Should transfer Notarization objects", async () => {
        await transferNotarization();
    });
    it("Should access ReadOnlyMethods", async () => {
        await accessReadOnlyMethods();
    });
});
