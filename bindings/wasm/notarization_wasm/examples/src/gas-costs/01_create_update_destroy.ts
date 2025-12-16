// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { State, NotarizationClient } from "@iota/notarization/node";
import { getFundedClient } from "../util";
import {EpochInfo, IotaTransactionBlockResponse} from "@iota/iota-sdk/client";

const STATE_METADATA: string | null = null; // "State metadata example";
const IMMUTABLE_DESCRIPTION: string | null  = null; // "This metadata will not change";

let EPOCH_INFO: EpochInfo | null = null;

const BILLION = 1000000000;

function print_gas_cost(transaction_type: String, flexDataSize: number, response: IotaTransactionBlockResponse) {
    const gasUsed = response.effects?.gasUsed;
    const referenceGasPrice = EPOCH_INFO ? EPOCH_INFO.referenceGasPrice ? parseInt(EPOCH_INFO.referenceGasPrice) : 1000 : 1000; // Fallback to 1000 if EpochInfo is not available

    if (gasUsed != undefined) {
        const totalGasCost = parseInt(gasUsed.computationCost) + parseInt(gasUsed.storageCost) - parseInt(gasUsed.storageRebate);
        const storageCost = parseInt(gasUsed.storageCost) / BILLION;
        const storageCostAboveMin = storageCost - 0.0029488;
        console.log("-------------------------------------------------------------------------------------------------------");
        console.log(`--- Gas cost for '${transaction_type}' transaction`);
        console.log("-------------------------------------------------------------------------------------------------------");
        console.log(`referenceGasPrice: ${referenceGasPrice / BILLION}`);
        console.log(`computationCost: ${parseInt(gasUsed.computationCost) / BILLION}`);
        console.log(`storageCost: ${storageCost}`);
        console.log(`flexDataSize: ${flexDataSize}`);
        console.log(`storageCost above minimum (0.0029488): ${storageCostAboveMin}`);
        console.log(`storageCostAboveMin per flexDataSize: ${storageCostAboveMin / (flexDataSize - 1)}`);
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

async function create_dynamic_notarization(notarizationClient: NotarizationClient, stateDataSize: number): Promise<{notarization: any, response: IotaTransactionBlockResponse}> {
    console.log(`Creating a dynamic notarization for state updates with ${stateDataSize} bytes of state data`);

    let stateData = randomString(stateDataSize)

    const {output: notarization, response: response} = await notarizationClient
        .createDynamic()
        .withStringState(stateData, STATE_METADATA)
        .withImmutableDescription(IMMUTABLE_DESCRIPTION)
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created dynamic notarization:", notarization.id);
    const flexDataSize = stateData.length + (STATE_METADATA ? STATE_METADATA.length : 0) + (IMMUTABLE_DESCRIPTION ? IMMUTABLE_DESCRIPTION.length : 0);
    print_gas_cost("Create", flexDataSize, response);

    return {notarization, response};
}

/** Create, update and destroy a Dynamic Notarization to estimate gas cost */
export async function createUpdateDestroy(): Promise<void> {
    console.log("Create, update and destroy a Dynamic Notarization to estimate gas cost");

    const notarizationClient = await getFundedClient();

    const iotaClient = notarizationClient.iotaClient();
    EPOCH_INFO = await iotaClient.getCurrentEpoch();
    console.log("Successfully fetched the EpochInfo to evaluate the referenceGasPrice: ", EPOCH_INFO != null ? EPOCH_INFO.referenceGasPrice : "Not Available");

    let notarization;

    // Create several dynamic notarizations with different initial state sizes. The notarization with the largest state size will be used for updates.
    console.log("\n🆕 Creating dynamic notarizations with different initial state sizes...");
    for (let i = 1; i <= 4; i++) {
        const result= await create_dynamic_notarization(notarizationClient, 10 * i*i); // 10, 40, 90, 160 bytes
        notarization = result.notarization;
    }


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
