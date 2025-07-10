// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { State } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrate how to update the state of a Notarization. */
export async function updateState(): Promise<void> {
    console.log("Demonstrating state updates on dynamic notarization");

    const notarizationClient = await getFundedClient();

    console.log("Creating a dynamic notarization for state updates...");

    // Create a dynamic notarization
    const { output: notarization } = await notarizationClient
        .createDynamic()
        .withStringState("Initial document version", "Version 1.0")
        .withImmutableDescription("Evolving document")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created dynamic notarization:", notarization.id);

    // Show initial state
    let notarizationClientReadOnly = notarizationClient.readOnly();
    const initialState = await notarizationClientReadOnly.state(notarization.id);

    const initialVersionCount = await notarizationClientReadOnly
        .stateVersionCount(notarization.id);

    console.log("\n📄 Initial State:");
    console.log("Content:", initialState.data.toString());
    console.log("Metadata:", initialState.metadata);
    console.log("Version count:", initialVersionCount);

    // Perform multiple state updates
    console.log("\n🔄 Performing state updates...");

    for (let i = 1; i <= 3; i++) {
        console.log(`\n--- Update ${i} ---`);

        // Create new state with updated content and metadata
        const newContent = `Updated document version ${i + 1}`;
        const newMetadata = `Version ${i + 1}.0 - Update ${i}`;

        // Update the state
        await notarizationClient
            .updateState(
                State.fromString(newContent, newMetadata),
                notarization.id,
            )
            .buildAndExecute(notarizationClient);

        console.log(`✅ State update ${i} completed`);

        // Verify the update
        const currentState = await notarizationClientReadOnly.state(notarization.id);

        const versionCount = await notarizationClientReadOnly
            .stateVersionCount(notarization.id);

        console.log("Updated content:", currentState.data.toString());
        console.log("Updated metadata:", currentState.metadata);
        console.log("New version count:", versionCount);

        // Verify version count incremented
        assert.equal(Number(versionCount), i);
    }

    // Show final version count
    const finalVersionCount = await notarizationClientReadOnly
        .stateVersionCount(notarization.id);

    const finalState = await notarizationClientReadOnly.state(notarization.id);

    console.log("\n📊 Final Statistics:");
    console.log("Total updates performed:", finalVersionCount);
    console.log("Final metadata:", finalState.metadata);

    // Get last state change timestamp
    const lastChange = await notarizationClientReadOnly
        .lastStateChangeTs(notarization.id);

    console.log("Last state change timestamp:", lastChange);

    console.log("\n🎯 Key Points:");
    console.log("✓ Dynamic notarizations support state updates");
    console.log("✓ Each update increments the version count");
    console.log("✓ Both text and bytes data are supported");
    console.log("✓ State metadata can be updated alongside content");
    console.log("✓ Timestamps track when changes were made");
}
