// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrate how to update the metadata of a Notarization. */
export async function updateMetadata(): Promise<void> {
    console.log("Demonstrating metadata updates on dynamic notarization");

    const notarizationClient = await getFundedClient();

    console.log("Creating a dynamic notarization for metadata updates...");

    // Create a dynamic notarization with initial updateable metadata
    const { output: notarization } = await notarizationClient
        .createDynamic()
        .withStringState("Document with evolving metadata", "State metadata")
        .withImmutableDescription("Document for metadata testing")
        .withUpdatableMetadata("Initial document metadata")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created dynamic notarization:", notarization.id);

    // Show initial metadata
    let notarizationClientReadOnly = notarizationClient.readOnly();
    const initialUpdatableMetadata = await notarizationClientReadOnly
        .updatableMetadata(notarization.id);

    const initialDescription = await notarizationClientReadOnly
        .description(notarization.id);

    const initialVersionCount = await notarizationClientReadOnly
        .stateVersionCount(notarization.id);

    console.log("\n📄 Initial Metadata:");
    console.log("Immutable description:", initialDescription);
    console.log("Updatable metadata:", initialUpdatableMetadata);
    console.log("State version count:", initialVersionCount);

    // Update metadata multiple times
    console.log("\n🔄 Performing metadata updates...");

    const metadataUpdates = [
        "Updated metadata - Phase 1",
        "Updated metadata - Phase 2",
        "Updated metadata - Phase 3",
        undefined, // Clear metadata, you can set it to null alternatively, but this will break the assertion below
        "Final metadata",
    ];

    for (let i = 0; i < metadataUpdates.length; i++) {
        console.log(`\n--- Metadata Update ${i + 1} ---`);

        const newMetadata = metadataUpdates[i];

        // Update the metadata
        await notarizationClient
            .updateMetadata(newMetadata, notarization.id)
            .buildAndExecute(notarizationClient);

        console.log(`✅ Metadata update ${i + 1} completed`);

        // Verify the update
        const currentMetadata = await notarizationClientReadOnly
            .updatableMetadata(notarization.id);

        const versionCount = await notarizationClientReadOnly
            .stateVersionCount(notarization.id);

        console.log("Updated metadata:", currentMetadata);
        console.log("Version count (should remain unchanged):", versionCount);

        // Verify that state version count doesn't change with metadata updates
        assert.equal(versionCount, initialVersionCount);

        // Verify metadata matches what we set
        assert.equal(currentMetadata, metadataUpdates[i]);
    }

    // Demonstrate that immutable description cannot be changed
    console.log("\n📋 Verifying immutable description remains unchanged...");

    const finalDescription = await notarizationClientReadOnly.description(notarization.id);

    console.log("Final description:", finalDescription);
    assert.equal(finalDescription, initialDescription);

    // Show that state content is unaffected by metadata updates
    console.log("\n📄 Verifying state content is unaffected...");

    const finalState = await notarizationClientReadOnly.state(notarization.id);

    console.log("State content:", finalState.data.toString());
    console.log("State metadata:", finalState.metadata);

    // Get timestamps to show metadata updates don't affect state change time
    const createdAt = await notarizationClientReadOnly.createdAtTs(notarization.id);

    const lastStateChange = await notarizationClientReadOnly
        .lastStateChangeTs(notarization.id);

    console.log("\n⏰ Timestamps:");
    console.log("Created at:", createdAt);
    console.log("Last state change:", lastStateChange);

    // Verify that lastStateChange timestamp equals the createdAt timestamp
    assert.equal(lastStateChange, createdAt);

    // Final metadata state
    const finalUpdatableMetadata = await notarizationClientReadOnly
        .updatableMetadata(notarization.id);

    console.log("\n📊 Final State:");
    console.log("Final updatable metadata:", finalUpdatableMetadata);

    const finalVersionCount = await notarizationClientReadOnly.stateVersionCount(notarization.id);
    console.log("State version count:", finalVersionCount);

    console.log("\n🎯 Key Points:");
    console.log("✓ Updatable metadata can be changed on dynamic notarizations");
    console.log("✓ Metadata updates don't affect state version count");
    console.log("✓ Metadata can be set to null or undefined (cleared)");
    console.log("✓ Immutable description cannot be changed after creation");
    console.log("✓ State content is unaffected by metadata updates");
    console.log("✓ Last state change timestamp is not updated by metadata changes");
}
