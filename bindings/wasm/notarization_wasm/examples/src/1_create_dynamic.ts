// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { NotarizationClientReadOnly } from "@iota/notarization-wasm";
import { IotaClient } from "@iota/iota-sdk/client";
import { getFundedClient, NETWORK_URL } from "./util";

/** Demonstrate how to create a Dynamic Notarization and publish it. */
export async function createDynamic(): Promise<void> {
    // create new client to connect to IOTA network
    const iotaClient = new IotaClient({ url: NETWORK_URL });
    const network = await iotaClient.getChainIdentifier();

    // create a new client that offers notarization related functions
    const notarizationClient = await getFundedClient();
/*
    // create a new Dynamic Notarization
    console.log("Creating new dynamic notarization");
    const { notarization: notarization } = await notarizationClient
         .createDynamic()
         .finish()
         .buildAndExecute(notarizationClient);

    // check if we can fetch the description via client
    console.log(`Notarization description: ${notarization.}`);

 */
}
