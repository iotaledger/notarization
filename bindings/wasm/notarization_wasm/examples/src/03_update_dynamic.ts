// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import {TimeLock, State} from "@iota/notarization-wasm";
import { IotaClient } from "@iota/iota-sdk/client";
import { getFundedClient, NETWORK_URL } from "./util";
import { strict as assert } from 'assert';

/** Demonstrate how to create a Dynamic Notarization, publish it and update its state. */
export async function updateDynamic(): Promise<void> {
    console.log("Demonstrating update on dynamic notarization")

    // create new client to connect to IOTA network
    const iotaClient = new IotaClient({ url: NETWORK_URL });
    // create a new client that offers notarization related functions
    const notarizationClient = await getFundedClient();

    // create a new Dynamic Notarization without transferLock
    console.log("Building a dynamic notarization and publish it to the IOTA network");
    const { output: notarization } = await notarizationClient
        .createDynamic()
        .withStringState("Initial live document content", "Version 1.0")
        .withImmutableDescription("All rights reserved to ACME Corp. No warranty provided.")
        .withUpdatableMetadata("For further information regarding the current document version please contact info@example.com.")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log(`\n✅ Dynamic notarization created with ID: ${notarization.id}`);

    // create a NotarizationClientReadOnly instance to read the notarization state and other data
    let notarizationReadOnly = notarizationClient.readOnly();

    // fetch the current state directly from the IOTA ledger using the `NotarizationClientReadOnly`
    let currentState = await notarizationReadOnly.state(notarization.id);
    console.log("Initial state:", currentState.data);
    console.log("Initial state metadata:", currentState.metadata);
    console.log("Initial state version count:", await notarizationReadOnly.stateVersionCount(notarization.id));

    // update the state on the the IOTA ledger using the `NotarizationClient`
    console.log("\n🔄 Updating state on dynamic notarization...");

    let newState = State.fromString("Updated document content", "Version 2.0");

    await notarizationClient
        .updateState(newState, notarization.id)
        .buildAndExecute(notarizationClient);

    console.log("✅ State update succeeded");

    currentState = await notarizationReadOnly.state(notarization.id);
    console.log("Updated state:", currentState.data);
    console.log("Updated state metadata:", currentState.metadata);
    let initialStateVersionCount = await notarizationReadOnly.stateVersionCount(notarization.id);
    console.log("Updated state version count:", initialStateVersionCount);


    // update the updatable metadata on the the IOTA ledger using the `NotarizationClient`
    console.log("\n\🔄 Updating updatable metadata on dynamic notarization...");
    console.log("Initial updatable metadata:", await notarizationReadOnly.updatableMetadata(notarization.id));

    console.log("\n📝 Updating metadata of the notarization...");
    await notarizationClient
        .updateMetadata("New contact email address: office@example.com", notarization.id)
        .buildAndExecute(notarizationClient);

    console.log("✅ Metadata update succeeded");

    console.log("New updatable metadata:", await notarizationReadOnly.updatableMetadata(notarization.id));

    // Check that the state version count did not change
    let currentStateVersionCount = await notarizationReadOnly.stateVersionCount(notarization.id);
    assert.strictEqual(initialStateVersionCount, currentStateVersionCount,
        "State version count should not change after updating updatable metadata");
    console.log("Current state version count is still:", currentStateVersionCount);

    // Summarize the dynamic notarization behaviour
    console.log("\n🔄 Dynamic notarizations are mutable - state-data, state-metadata and updatable metadata can be changed");
    console.log("🔄 Updatable metadata can only be changed for dynamic notarizations");
    console.log("🔒 Updating the updatable metadata doesn't effect the stateVersionCount");
}