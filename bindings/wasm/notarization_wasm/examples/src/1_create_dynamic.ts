// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import {TimeLock} from "@iota/notarization-wasm";

import { IotaClient } from "@iota/iota-sdk/client";
import { getFundedClient, NETWORK_URL } from "./util";

/** Demonstrate how to create a Dynamic Notarization and publish it. */
export async function createDynamic(): Promise<void> {
    console.log("Creating a simple dynamic notarization example");

    // create new client to connect to IOTA network
    const iotaClient = new IotaClient({ url: NETWORK_URL });
    const network = await iotaClient.getChainIdentifier();

    // create a new client that offers notarization related functions
    const notarizationClient = await getFundedClient();

    let utf8Encode = new TextEncoder();

    // create a new Dynamic Notarization
    console.log("Building a dynamic notarization and publish it to the IOTA network");
    const { output: notarization } = await notarizationClient
        .createDynamic()
        // Control the type of State data by choosing one of the `with...State` functions below.
        // Uncomment or comment the following lines to choose between string or byte State data.
        //.withStringState("HelloWorld")
        //.withBytesState(utf8Encode.encode("HelloWorld"), "Data description goes here")
        .withBytesState(Uint8Array.from([14,255,0,125,64,87,11,114,108,100]), "Data description may be used for version specifiers")
        .withTransferLock(TimeLock.withUnlockAt(1814399999))
        .withImmutableDescription("This can not be changed any more")
        .withUpdateableMetadata("This can be updated")
         .finish()
         .buildAndExecute(notarizationClient);

    console.log("\n✅ Dynamic notarization created successfully!");

    // check some important properties of the received OnChainNotarization
    console.log("\n----------------------------------------------------");
    console.log("----- Important Notarization Properties ------------");
    console.log("----------------------------------------------------");
    console.log("Notarization ID: ", notarization.id);
    console.log("Notarization Method: ", notarization.method);
    console.log(`State data as string: "${notarization.state.data.toString()}" or as bytes: [${notarization.state.data.toBytes()}]` );
    console.log("State metadata: ", notarization.state.metadata);
    console.log("Immutable description: ", notarization.immutableMetadata.description);
    console.log("Immutable locking metadata: ", notarization.immutableMetadata.locking);
    console.log("Updateable metadata: ", notarization.updateableMetadata);
    console.log("State version count: ", notarization.description);

    // This is how the complete OnChainNotarization looks like
    console.log("\n----------------------------------------------------");
    console.log("----- All Notarization Properties      -------------");
    console.log("----------------------------------------------------");
    console.log("Notarization: ", notarization);

}
