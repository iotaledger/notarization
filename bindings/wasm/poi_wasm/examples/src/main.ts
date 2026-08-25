// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createAndVerifyTransactionProof } from "./01_transaction_proof.js";
import { createAndVerifyMultiTargetProof } from "./02_multi_target_proof.js";
import { reuseVerifierForMultipleProofs } from "./03_reuse_verifier.js";
import { createAndVerifyObjectProof } from "./04_object_proof.js";
import { createAndVerifyEventProof } from "./05_event_proof.js";

export async function main(example?: string): Promise<void> {
    const argument = example ?? process.argv[2]?.toLowerCase();
    if (!argument) {
        throw new Error("Please specify an example name, e.g. '01_transaction_proof'");
    }

    switch (argument) {
        case "01_transaction_proof":
            return createAndVerifyTransactionProof();
        case "02_multi_target_proof":
            return createAndVerifyMultiTargetProof();
        case "03_reuse_verifier":
            return reuseVerifierForMultipleProofs();
        case "04_object_proof":
            return createAndVerifyObjectProof();
        case "05_event_proof":
            return createAndVerifyEventProof();
        default:
            throw new Error(`Unknown example name: '${argument}'`);
    }
}

main().catch((error: unknown) => {
    console.error("Example error:", error);
    process.exitCode = 1;
});
