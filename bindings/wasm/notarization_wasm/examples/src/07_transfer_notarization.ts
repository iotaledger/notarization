// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { Ed25519Keypair } from "@iota/iota-sdk/keypairs/ed25519";
import { TimeLock } from "@iota/notarization/node";
import { strict as assert } from "assert";
import { getFundedClient } from "./util";

/** Demonstrate how to transfer a dynamic Notarization and transferring a locked Notarization will fail. */
export async function transferNotarization(): Promise<void> {
    console.log("Demonstrating notarization transfer scenarios");

    const notarizationClient = await getFundedClient();

    // Generate random addresses for transfer recipients
    const alice = Ed25519Keypair.generate().toIotaAddress();
    const bob = Ed25519Keypair.generate().toIotaAddress();

    console.log("Transfer recipients:");
    console.log("Alice:", alice.toString());
    console.log("Bob:", bob.toString());

    // Scenario 1: Transfer an unlocked dynamic notarization (should succeed)
    console.log("\n📝 Scenario 1: Creating and transferring an unlocked dynamic notarization...");

    const { output: unlocked } = await notarizationClient
        .createDynamic()
        .withStringState("Transferable document", "Transfer test")
        .withImmutableDescription("Unlocked dynamic document")
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created unlocked dynamic notarization:", unlocked.id);

    // Check transfer lock status
    let notarizationClientReadOnly = notarizationClient.readOnly();
    const isTransferLocked = await notarizationClientReadOnly
        .isTransferLocked(unlocked.id);

    console.log("🔍 Transfer locked:", isTransferLocked);

    // Transfer the unlocked notarization to Alice
    try {
        await notarizationClient
            .transferNotarization(unlocked.id, alice)
            .buildAndExecute(notarizationClient);
        console.log("✅ Successfully transferred unlocked notarization to Alice");
    } catch (e) {
        console.log("❌ Failed to transfer:", e);
        assert.fail("Transfer of unlocked notarization should succeed");
    }

    // Scenario 2: Try to transfer a transfer-locked dynamic notarization (should fail)
    console.log("\n📝 Scenario 2: Creating a transfer-locked dynamic notarization...");

    const now = Math.round(Date.now() / 1000);
    const unlockAt = now + 3600; // 1 hour

    const { output: transferLocked } = await notarizationClient
        .createDynamic()
        .withStringState("Transfer-locked document", undefined)
        .withImmutableDescription("Transfer-locked document")
        .withTransferLock(TimeLock.withUnlockAt(unlockAt))
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created transfer-locked dynamic notarization:", transferLocked.id);

    const isTransferLockedStatus = await notarizationClientReadOnly
        .isTransferLocked(transferLocked.id);

    console.log("🔍 Transfer locked:", isTransferLockedStatus);

    // Try to transfer the locked notarization
    try {
        await notarizationClient
            .transferNotarization(transferLocked.id, bob)
            .buildAndExecute(notarizationClient);
        assert.fail("❌ Unexpected: Transfer succeeded (should have failed)");
    } catch (e) {
        console.log("✅ Expected: Transfer failed -", e);
    }

    // Scenario 3: Try to transfer a locked notarization (should always fail)
    console.log("\n📝 Scenario 3: Creating a locked notarization...");

    const { output: locked } = await notarizationClient
        .createLocked()
        .withStringState("Locked document content", undefined)
        .withImmutableDescription("Locked document")
        .withDeleteLock(TimeLock.withNone())
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Created locked notarization:", locked.id);

    const isLockedStatusTransferLocked = await notarizationClientReadOnly
        .isTransferLocked(locked.id);

    console.log("🔍 Transfer locked:", isLockedStatusTransferLocked);

    // Try to transfer the locked notarization
    try {
        await notarizationClient
            .transferNotarization(locked.id, alice)
            .buildAndExecute(notarizationClient);
        assert.fail("❌ Unexpected: Transfer succeeded (should have failed)");
    } catch (e) {
        console.log("✅ Expected: Transfer failed -", e);
    }

    // Show lock metadata for different scenarios
    console.log("\n🔐 Lock Metadata Analysis:");

    const unlockedLockMetadata = await notarizationClientReadOnly
        .lockMetadata(unlocked.id);

    const transferLockedLockMetadata = await notarizationClientReadOnly
        .lockMetadata(transferLocked.id);

    const lockedLockMetadata = await notarizationClientReadOnly
        .lockMetadata(locked.id);

    console.log("Unlocked notarization lock metadata:", unlockedLockMetadata);
    console.log("Transfer-locked notarization lock metadata:", transferLockedLockMetadata);
    console.log("Locked notarization lock metadata:", lockedLockMetadata);

    console.log("\n📋 Transfer Rules Summary:");
    console.log("✅ Unlocked dynamic notarizations can be transferred freely");
    console.log("🔒 Transfer-locked dynamic notarizations cannot be transferred until lock expires");
    console.log("🚫 Locked notarizations can never be transferred (transfer_lock = UntilDestroyed)");
    console.log("⏰ Transfer locks are time-based and will expire automatically");
    console.log("🔍 Use isTransferLocked() to check transfer status before attempting");

    // Demonstrate checking multiple transfer statuses
    console.log("\n🔍 Final Transfer Status Check:");

    const statuses = [
        { name: "Unlocked", id: unlocked.id },
        { name: "Transfer-locked", id: transferLocked.id },
        { name: "Locked", id: locked.id },
    ];

    for (const { name, id } of statuses) {
        const isLocked = await notarizationClientReadOnly.isTransferLocked(id);
        console.log(`${name}: Transfer locked = ${isLocked}`);
    }
}
