// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createLocked } from "./01_create_locked";
import { createDynamic } from "./02_create_dynamic";
import { updateDynamic } from "./03_update_dynamic";

export async function main(example?: string) {
    // Extract example name.
    const argument = example ?? process.argv?.[2]?.toLowerCase();
    if (!argument) {
        throw "Please specify an example name, e.g. '0_create_did'";
    }

    switch (argument) {
        case "01_create_locked":
            return await createLocked();
        case "02_create_dynamic":
            return await createDynamic();
        case "03_update_dynamic":
            return await updateDynamic();
        default:
            throw "Unknown example name: '" + argument + "'";
    }
}

main()
    .catch((error) => {
        console.log("Example error:", error);
    });
