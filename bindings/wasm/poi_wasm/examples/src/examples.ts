// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createAndVerifyTransactionProof } from "./01_transaction_proof.js";
import { createAndVerifyMultiTargetProof } from "./02_multi_target_proof.js";
import { reuseVerifierForMultipleProofs } from "./03_reuse_verifier.js";
import { createAndVerifyObjectProof } from "./04_object_proof.js";
import { createAndVerifyEventProof } from "./05_event_proof.js";

interface PoiExample {
    readonly testName: string;
    readonly run: () => Promise<void>;
}

/** Runnable Proof of Inclusion examples keyed by their command-line names. */
export const examples: Readonly<Record<string, PoiExample>> = {
    "01_transaction_proof": {
        testName: "creates and verifies a transaction proof",
        run: createAndVerifyTransactionProof,
    },
    "02_multi_target_proof": {
        testName: "creates and verifies a multi-target proof",
        run: createAndVerifyMultiTargetProof,
    },
    "03_reuse_verifier": {
        testName: "reuses a verifier for multiple proofs",
        run: reuseVerifierForMultipleProofs,
    },
    "04_object_proof": {
        testName: "creates and verifies an object proof",
        run: createAndVerifyObjectProof,
    },
    "05_event_proof": {
        testName: "creates and verifies an event proof",
        run: createAndVerifyEventProof,
    },
};
