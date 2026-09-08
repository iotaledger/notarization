// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { examples } from "./examples.js";

export async function main(exampleName?: string): Promise<void> {
    const selectedName = exampleName ?? process.argv[2]?.toLowerCase();
    if (!selectedName) {
        throw new Error("Please specify an example name, e.g. '01_transaction_proof'");
    }

    const selectedExample = examples[selectedName];
    if (!selectedExample) {
        throw new Error(`Unknown example name: '${selectedName}'`);
    }

    await selectedExample.run();
}

main().catch((error: unknown) => {
    console.error("Example error:", error);
    process.exitCode = 1;
});
