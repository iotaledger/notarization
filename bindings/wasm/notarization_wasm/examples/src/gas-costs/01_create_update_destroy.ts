// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { State } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "../util";
import {IotaTransactionBlockResponse} from "@iota/iota-sdk/client";

const STATE_DATA = "1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111"; //"This is some state data";
const STATE_METADATA: string | null = null; // "State metadata example";
const IMMUTABLE_DESCRIPTION: string | null  = null; // "This metadata will not change";


const BILLION = 1000000000;

function print_gas_cost(transaction_type: String, flexDataSize: number, response: IotaTransactionBlockResponse) {
    const gasUsed = response.effects?.gasUsed;

    if (gasUsed != undefined) {
        const totalGasCost = parseInt(gasUsed.computationCost) + parseInt(gasUsed.storageCost) - parseInt(gasUsed.storageRebate);
        const storageCost = parseInt(gasUsed.storageCost) / BILLION;
        const storageCostAboveMin = storageCost - 0.0029488;
        console.log("-------------------------------------------------------------------------------------------------------");
        console.log(`--- Gas cost for '${transaction_type}' transaction`);
        console.log("-------------------------------------------------------------------------------------------------------");
        console.log(`computationCost: ${parseInt(gasUsed.computationCost) / BILLION}`);
        console.log(`storageCost: ${storageCost}`);
        console.log(`flexDataSize: ${flexDataSize}`);
        console.log(`storageCost above minimum (0.0029488): ${storageCostAboveMin}`);
        console.log(`storageCostAboveMin per flexDataSize: ${storageCostAboveMin / flexDataSize}`);
        console.log(`storageRebate: ${parseInt(gasUsed.storageRebate) / BILLION}`);
        console.log(`totalGasCost (calculated): ${totalGasCost / BILLION}`);
        console.log("-------------------------------------------------------------------------------------------------------");
    } else {
        console.log("Gas used information is not available.");
    }
}

function randomString(length = 50) {
    return [...Array(length + 10)].map((value) => (Math.random() * 1000000).toString(36).replace('.', '')).join('').substring(0, length);
};

/** Create, update and destroy a Dynamic Notarization to estimate gas cost */
export async function createUpdateDestroy(): Promise<void> {
    console.log("Create, update and destroy a Dynamic Notarization to estimate gas cost");

    const notarizationClient = await getFundedClient();

    console.log("Creating a dynamic notarization for state updates...");

    // Create a dynamic notarization
    const { output: notarization, response: response } = await notarizationClient
        .createDynamic()
        .withStringState(STATE_DATA, STATE_METADATA)
        .withImmutableDescription(IMMUTABLE_DESCRIPTION)
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created dynamic notarization:", notarization.id);
    const flexDataSize = STATE_DATA.length + (STATE_METADATA ? STATE_METADATA.length : 0) + (IMMUTABLE_DESCRIPTION ? IMMUTABLE_DESCRIPTION.length : 0);
    print_gas_cost("Create", flexDataSize, response);

    // Perform multiple state updates
    console.log("\n🔄 Performing state updates...");

    for (let i = 1; i <= 3; i++) {
        console.log(`\n--- Update ${i} ---`);

        // Create new state with updated content and metadata
        const newContent = randomString(i * 50);
        const newMetadata = `Version ${i + 1}.0 - Update ${i}`;

        // Update the state
        const { output: _, response: response } = await  notarizationClient
            .updateState(
                State.fromString(newContent, newMetadata),
                notarization.id,
            )
            .buildAndExecute(notarizationClient);

        console.log(`✅ State update ${i} completed`);
        const flexDataSize = newContent.length + (STATE_METADATA ? newMetadata.length : 0);
        print_gas_cost("Update", flexDataSize, response);
    }

    // Destroy the dynamic notarization
    try {
        const { output: _, response: response } = await notarizationClient
            .destroy(notarization.id)
            .buildAndExecute(notarizationClient);
        console.log("✅ Successfully destroyed unlocked dynamic notarization");
        print_gas_cost("Destroy", 1, response);
    } catch (e) {
        console.log("❌ Failed to destroy:", e);
    }
}
