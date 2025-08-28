// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { State, TimeLock } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrates read-only methods for notarization inspection. */
export async function accessReadOnlyMethods(): Promise<void> {
    console.log("Demonstrating read-only methods for notarization inspection");

    const notarizationClient = await getFundedClient();

    // Create a comprehensive dynamic notarization for testing
    console.log("Creating a dynamic notarization with comprehensive metadata...");

    const description = "Comprehensive test document";
    const updatableMetadata = "Initial document metadata";

    const { output: dynamicNotarization } = await notarizationClient
        .createDynamic()
        .withStringState("Document content with detailed metadata", "State-level metadata")
        .withImmutableDescription(description)
        .withUpdatableMetadata(updatableMetadata)
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created dynamic notarization:", dynamicNotarization.id);

    // Demonstrate all read-only methods for dynamic notarization
    console.log("\n📖 Read-only Methods for Dynamic Notarization:");

    // 1. Get description (immutable)
    let notarizationClientReadOnly = notarizationClient.readOnly();
    const retrievedDescription = await notarizationClientReadOnly
        .description(dynamicNotarization.id);
    console.log("📝 Description:", retrievedDescription);

    // 2. Get updateable metadata
    const retrievedMetadata = await notarizationClientReadOnly
        .updatableMetadata(dynamicNotarization.id);
    console.log("📋 Updatable metadata:", retrievedMetadata);

    // 3. Get current state
    const currentState = await notarizationClientReadOnly.state(dynamicNotarization.id);
    console.log("📄 State content:", currentState.data.toString());
    console.log("📄 State metadata:", currentState.metadata);

    // 4. Get creation timestamp
    const createdAt = await notarizationClientReadOnly
        .createdAtTs(dynamicNotarization.id);
    console.log("🕐 Created at timestamp:", createdAt);

    // 5. Get last state change timestamp
    const lastStateChange = await notarizationClientReadOnly
        .lastStateChangeTs(dynamicNotarization.id);
    console.log("🕐 Last state change timestamp:", lastStateChange);
    assert(createdAt === lastStateChange, "createdAt timestamp must equal last state change after initial creation");

    // 6. Get state version count
    const versionCount = await notarizationClientReadOnly
        .stateVersionCount(dynamicNotarization.id);
    console.log("🔢 State version count:", versionCount);
    assert(versionCount === 0n, "versionCount must be 0n after initial creation");

    // 7. Get notarization method
    const method = await notarizationClientReadOnly
        .notarizationMethod(dynamicNotarization.id);
    console.log("⚙️ Notarization method:", method);
    assert(method === "Dynamic", "method of a dynamic Notarization must be 'Dynamic'");

    // 8. Check lock statuses
    const isTransferLocked = await notarizationClientReadOnly
        .isTransferLocked(dynamicNotarization.id);
    const isUpdateLocked = await notarizationClientReadOnly
        .isUpdateLocked(dynamicNotarization.id);
    const isDestroyAllowed = await notarizationClientReadOnly
        .isDestroyAllowed(dynamicNotarization.id);
    console.log("🔒 Transfer locked:", isTransferLocked);
    assert(!isTransferLocked, "Per default a dynamic Notarization must be not transfer locked");
    console.log("🔒 Update locked:", isUpdateLocked);
    assert(!isUpdateLocked, "Per default a dynamic Notarization must be not update locked");
    console.log("🗑️ Destroy allowed:", isDestroyAllowed);
    assert(isDestroyAllowed, "Per default deleting a dynamic Notarization shall be allowed");

    // 9. Get lock metadata
    const lockMetadata = await notarizationClientReadOnly
        .lockMetadata(dynamicNotarization.id);
    console.log("🔐 Lock metadata:", lockMetadata);
    assert(lockMetadata === undefined, "Per default a dynamic Notarization has no lock metadata");

    // 10. Get the whole OnChainNotarization at once and pretty print it
    const onChainNotarization = await notarizationClientReadOnly
        .getNotarizationById(dynamicNotarization.id);
    console.log("📦 Complete dynamic OnChainNotarization:", onChainNotarization);

    // Update the state to demonstrate version tracking
    console.log("\n🔄 Updating state to demonstrate version tracking...");

    const newState = State.fromString(
        "Updated document content",
        "Updated state metadata",
    );

    await notarizationClient
        .updateState(newState, dynamicNotarization.id)
        .buildAndExecute(notarizationClient);

    // Show updated read-only values
    console.log("\n📊 After State Update:");

    const updatedVersionCount = await notarizationClientReadOnly
        .stateVersionCount(dynamicNotarization.id);
    const updatedLastChange = await notarizationClientReadOnly
        .lastStateChangeTs(dynamicNotarization.id);
    const updatedState = await notarizationClientReadOnly.state(dynamicNotarization.id);

    console.log("🔢 New version count:", updatedVersionCount);
    assert(updatedVersionCount === 1n, "versionCount must be 1n after first state update");
    console.log("🕐 Updated last change timestamp:", updatedLastChange);
    assert(
        createdAt < updatedLastChange,
        "createdAt timestamp must lower lastStateChange timestamp after first state update",
    );
    console.log("📄 Updated state content:", updatedState.data.toString());
    assert(
        updatedState.data.toString() !== currentState.data.toString(),
        "Intial State data must differ from current State data after first state update",
    );

    // Create a locked notarization for comparison
    console.log("\n🔒 Creating a locked notarization for comparison...");

    const now = Math.round(Date.now() / 1000);
    const unlockAt = now + 86400; // 24 hours

    const { output: lockedNotarization } = await notarizationClient
        .createLocked()
        .withStringState("Locked document content", "Locked state metadata")
        .withImmutableDescription("Locked test document")
        .withUpdatableMetadata("Locked document metadata")
        .withDeleteLock(TimeLock.withUnlockAt(unlockAt))
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created locked notarization:", lockedNotarization.id);

    // Demonstrate read-only methods for locked notarization
    console.log("\n📖 Read-only Methods for Locked Notarization:");

    const lockedMethod = await notarizationClientReadOnly
        .notarizationMethod(lockedNotarization.id);
    const lockedTransferLocked = await notarizationClientReadOnly
        .isTransferLocked(lockedNotarization.id);
    const lockedUpdateLocked = await notarizationClientReadOnly
        .isUpdateLocked(lockedNotarization.id);
    const lockedDestroyAllowed = await notarizationClientReadOnly
        .isDestroyAllowed(lockedNotarization.id);
    const lockedLockMetadata = await notarizationClientReadOnly
        .lockMetadata(lockedNotarization.id);
    const lockedOnChainNotarization = await notarizationClientReadOnly
        .getNotarizationById(lockedNotarization.id);
    console.log("⚙️ Method:", lockedMethod);
    assert(lockedMethod === "Locked", "method of a locked Notarization must be 'Locked'");
    console.log("🔒 Transfer locked:", lockedTransferLocked);
    assert(lockedTransferLocked, "A locked Notarization must be transfer locked");
    console.log("🔒 Update locked:", lockedUpdateLocked);
    assert(lockedUpdateLocked, "A locked Notarization must be update locked");
    console.log("🗑️ Destroy allowed:", lockedDestroyAllowed);
    assert(!lockedDestroyAllowed, "Destroying a delete-locked locked Notarization must be forbidden");
    console.log("🔐 Lock metadata present:", lockedLockMetadata !== undefined);
    assert(lockedLockMetadata !== undefined, "A locked Notarization must have lock metadata");
    console.log("📦 Complete locked OnChainNotarization:", lockedOnChainNotarization);

    // Compare methods between dynamic and locked
    console.log("\n📊 Comparison Summary:");
    console.log("┌─────────────────────┬─────────────┬─────────────┐");
    console.log("│ Property            │ Dynamic     │ Locked      │");
    console.log("├─────────────────────┼─────────────┼─────────────┤");
    console.log(`│ Method              │ ${String(method).padEnd(11)} │ ${String(lockedMethod).padEnd(11)} │`);
    console.log(
        `│ Transfer Locked     │ ${String(isTransferLocked).padEnd(11)} │ ${String(lockedTransferLocked).padEnd(11)} │`,
    );
    console.log(
        `│ Update Locked       │ ${String(isUpdateLocked).padEnd(11)} │ ${String(lockedUpdateLocked).padEnd(11)} │`,
    );
    console.log(
        `│ Destroy Allowed     │ ${String(isDestroyAllowed).padEnd(11)} │ ${String(lockedDestroyAllowed).padEnd(11)} │`,
    );
    console.log(
        `│ Has Lock Metadata   │ ${String(lockMetadata !== undefined).padEnd(11)} │ ${
            String(lockedLockMetadata !== undefined).padEnd(11)
        } │`,
    );
    console.log("└─────────────────────┴─────────────┴─────────────┘");

    console.log("\n🎯 Key Points about Read-only Methods:");
    console.log("✓ All notarizations support the same read-only interface");
    console.log("✓ State version count tracks state updates (not metadata updates)");
    console.log("✓ Timestamps help track creation and modification times");
    console.log("✓ Lock checking methods help determine allowed operations");
    console.log("✓ Dynamic and locked notarizations have different lock behaviors");
    console.log("✓ Lock metadata provides detailed information about applied locks");
}
