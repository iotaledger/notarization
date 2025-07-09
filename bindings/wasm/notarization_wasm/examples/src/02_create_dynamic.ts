// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { TimeLock } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrate how to create a Dynamic Notarization and publish it. */
export async function createDynamic(): Promise<void> {
    console.log("Creating a simple dynamic notarization example");

    // create a new client that offers notarization related functions
    const notarizationClient = await getFundedClient();

    // Calculate an unlock time (24 hours from now) to be used for transferLock
    const transfer_unlock_at = Math.round(Date.now() / 1000 + 86400); // 24 hours

    const utf8Encode = new TextEncoder();

    // create a new Dynamic Notarization
    console.log("Building a dynamic notarization with transferLock and publish it to the IOTA network");
    const { output: notarization } = await notarizationClient
        .createDynamic()
        // Control the type of State data by choosing one of the `with...State` functions below.
        // Uncomment or comment the following lines to choose between string or byte State data.
        //
        // .withStringState("Live document content", "Version 1.0")
        // .withBytesState(utf8Encode.encode("Live document content"), "Version 1.0")
        .withBytesState(Uint8Array.from([14, 255, 0, 125, 64, 87, 11, 114, 108, 100]), "Version 1.0")
        .withTransferLock(TimeLock.withUnlockAt(transfer_unlock_at))
        .withImmutableDescription("This can not be changed any more")
        .withUpdatableMetadata("This can be updated")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("\n✅ Successfully created a transfer locked Dynamic notarization!");

    // check some important properties of the received OnChainNotarization
    console.log("\n----------------------------------------------------");
    console.log("----- Important Notarization Properties ------------");
    console.log("----------------------------------------------------");
    console.log("Notarization ID: ", notarization.id);
    console.log("Notarization Method: ", notarization.method);
    console.log(
        `State data as string: "${notarization.state.data.toString()}" or as bytes: [${notarization.state.data.toBytes()}]`,
    );
    console.log("State metadata: ", notarization.state.metadata);
    console.log("Immutable description: ", notarization.immutableMetadata.description);
    console.log("Immutable locking metadata: ", notarization.immutableMetadata.locking);
    console.log("Updatable metadata: ", notarization.updatableMetadata);
    console.log("State version count: ", notarization.stateVersionCount);

    // This is what the complete OnChainNotarization looks like
    console.log("\n----------------------------------------------------");
    console.log("----- All Notarization Properties      -------------");
    console.log("----------------------------------------------------");
    console.log("Notarization: ", notarization);

    // Verify the notarization method is Dynamic
    assert(notarization.method === "Dynamic");

    // Check if it has locking metadata and `transferLock` is set to `UnlockAt` using the right argument
    assert(notarization.immutableMetadata.locking !== undefined);
    assert(notarization.immutableMetadata.locking.transferLock.type == "UnlockAt");
    assert(notarization.immutableMetadata.locking.transferLock.args === transfer_unlock_at);

    console.log("\n🔄 The notarization is Dynamic and can be updated at any time");
    console.log(
        `🔒 The notarization cannot be transferred or destroyed until the transfer lock ${transfer_unlock_at} expires`,
    );
}
