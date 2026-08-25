// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, it } from "node:test";

import { createAndVerifyTransactionProof } from "./01_create_and_verify_transaction_proof.js";
import { createAndVerifyMultiTargetProof } from "./02_create_and_verify_multi_target_proof.js";
import { reuseVerifierForMultipleProofs } from "./03_reuse_verifier_for_multiple_proofs.js";
import { createAndVerifyObjectProof } from "./04_create_and_verify_object_proof.js";
import { createAndVerifyEventProof } from "./05_create_and_verify_event_proof.js";

describe("Proof of Inclusion wasm node examples", () => {
    afterEach(() => {
        console.log("\n----------------------------------------------------\n");
    });

    it("creates and verifies a transaction proof", async () => {
        await createAndVerifyTransactionProof();
    });
    it("creates and verifies a multi-target proof", async () => {
        await createAndVerifyMultiTargetProof();
    });
    it("reuses a verifier for multiple proofs", async () => {
        await reuseVerifierForMultipleProofs();
    });
    it("creates and verifies an object proof", async () => {
        await createAndVerifyObjectProof();
    });
    it("creates and verifies an event proof", async () => {
        await createAndVerifyEventProof();
    });
});
