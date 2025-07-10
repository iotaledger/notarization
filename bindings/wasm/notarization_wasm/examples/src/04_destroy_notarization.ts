// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { TimeLock } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrate how to destroy a Notarization. */
export async function destroyNotarization(): Promise<void> {
    console.log("Demonstrating notarization destruction scenarios");

    // Create a notarization client
    const notarizationClient = await getFundedClient();

    // Scenario 1: Destroy an unlocked dynamic notarization (should succeed)
    console.log("\n📝 Scenario 1: Creating and destroying an unlocked dynamic notarization...");

    const { output: unlocked } = await notarizationClient
        .createDynamic()
        .withStringState("Unlocked content", undefined)
        .withImmutableDescription("Unlocked dynamic document")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created unlocked dynamic notarization:", unlocked.id);

    // Check if destroy is allowed
    // create a NotarizationClientReadOnly instance to read the notarization state and other data
    const notarizationClientReadOnly = notarizationClient.readOnly();
    const isDestroyAllowed = await notarizationClientReadOnly.isDestroyAllowed(unlocked.id);
    console.log("🔍 Destroy allowed:", isDestroyAllowed);

    // Destroy the unlocked notarization
    try {
        await notarizationClient
            .destroy(unlocked.id)
            .buildAndExecute(notarizationClient);
        console.log("✅ Successfully destroyed unlocked dynamic notarization");
    } catch (e) {
        console.log("❌ Failed to destroy:", e);
        assert.fail("❌ Unexpected: Destruction failed");
    }

    // Scenario 2: Try to destroy a transfer-locked dynamic notarization (should fail)
    console.log("\n📝 Scenario 2: Creating a transfer-locked dynamic notarization...");

    const now = Math.round(Date.now() / 1000);
    const unlockAt = now + 86400; // 24 hours

    const { output: transferLocked } = await notarizationClient
        .createDynamic()
        .withStringState("Transfer-locked content", undefined)
        .withImmutableDescription("Transfer-locked document")
        .withTransferLock(TimeLock.withUnlockAt(unlockAt))
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created transfer-locked dynamic notarization:", transferLocked.id);

    const isTransferLockedDestroyAllowed = await notarizationClientReadOnly.isDestroyAllowed(transferLocked.id);
    console.log("🔍 Destroy allowed:", isTransferLockedDestroyAllowed);

    // Try to destroy the transfer-locked notarization
    try {
        await notarizationClient
            .destroy(transferLocked.id)
            .buildAndExecute(notarizationClient);
        assert.fail("❌ Unexpected: Destruction succeeded (should have failed)");
    } catch (e) {
        console.log("✅ Expected: Destruction failed -", e);
    }

    // Scenario 3: Create and try to destroy a time-locked locked notarization (should fail)
    console.log("\n📝 Scenario 3: Creating a time-locked locked notarization...");

    const { output: deleteLocked } = await notarizationClient
        .createLocked()
        .withStringState("Delete-locked content", undefined)
        .withImmutableDescription("Delete-locked document")
        .withDeleteLock(TimeLock.withUnlockAt(unlockAt))
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created delete-locked locked notarization:", deleteLocked.id);

    const isDeleteLockedDestroyAllowed = await notarizationClientReadOnly.isDestroyAllowed(deleteLocked.id);
    console.log("🔍 Destroy allowed:", isDeleteLockedDestroyAllowed);

    // Try to destroy the delete-locked notarization
    try {
        await notarizationClient
            .destroy(deleteLocked.id)
            .buildAndExecute(notarizationClient);
        assert.fail("❌ Unexpected: Destruction succeeded (should have failed)");
    } catch (e) {
        console.log("✅ Expected: Destruction failed -", e);
    }

    // Scenario 4: Create and destroy a locked notarization with no delete lock (should succeed)
    console.log("\n📝 Scenario 4: Creating a locked notarization with no delete lock...");

    const { output: noDeleteLock } = await notarizationClient
        .createLocked()
        .withStringState("No delete lock content", undefined)
        .withImmutableDescription("No delete lock document")
        .withDeleteLock(TimeLock.withNone())
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created locked notarization with no delete lock:", noDeleteLock.id);

    const isNoDeleteLockDestroyAllowed = await notarizationClientReadOnly.isDestroyAllowed(noDeleteLock.id);
    console.log("🔍 Destroy allowed:", isNoDeleteLockDestroyAllowed);

    // Destroy the notarization with no delete lock
    try {
        await notarizationClient
            .destroy(noDeleteLock.id)
            .buildAndExecute(notarizationClient);
        console.log("✅ Successfully destroyed locked notarization with no delete lock");
    } catch (e) {
        console.log("❌ Failed to destroy:", e);
        assert.fail("❌ Unexpected: Destruction failed");
    }

    console.log("\n📋 Summary:");
    console.log("🔓 Unlocked notarizations can be destroyed immediately");
    console.log("🔒 Transfer-locked dynamic notarizations cannot be destroyed");
    console.log("⏰ Time-locked locked notarizations cannot be destroyed before lock expires");
    console.log("🆓 Locked notarizations with TimeLock::None can be destroyed");
}
