// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createLocked } from "./01_create_locked";
import { createDynamic } from "./02_create_dynamic";
import { updateDynamic } from "./03_update_dynamic";
import { destroyNotarization } from "./04_destroy_notarization";
import { updateState } from "./05_update_state";
import { updateMetadata } from "./06_update_metadata";
import { transferNotarization } from "./07_transfer_notarization";
import { accessReadOnlyMethods } from "./08_access_read_only_methods";

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
        case "04_destroy_notarization":
            return await destroyNotarization();
        case "05_update_state":
            return await updateState();
        case "06_update_metadata":
            return await updateMetadata();
        case "07_transfer_notarization":
            return await transferNotarization();
        case "08_access_read_only_methods":
            return await accessReadOnlyMethods();
        default:
            throw "Unknown example name: '" + argument + "'";
    }
}

main()
    .catch((error) => {
        console.log("Example error:", error);
    });
