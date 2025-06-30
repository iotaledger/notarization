// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use iota_sdk::types::base_types::IotaAddress;
use notarization::core::types::{State, TimeLock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating notarization transfer scenarios");

    let notarization_client = get_funded_client().await?;

    // Generate random addresses for transfer recipients
    let alice = IotaAddress::random_for_testing_only();
    let bob = IotaAddress::random_for_testing_only();

    println!("Transfer recipients:");
    println!("Alice: {alice}");
    println!("Bob: {bob}");

    // Scenario 1: Transfer an unlocked dynamic notarization (should succeed)
    println!("\n📝 Scenario 1: Creating and transferring an unlocked dynamic notarization...");

    let unlocked_notarization_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Transferable document".to_string(),
            Some("Transfer test".to_string()),
        ))
        .with_immutable_description("Unlocked dynamic document".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created unlocked dynamic notarization: {unlocked_notarization_id:?}");

    // Check transfer lock status
    let is_transfer_locked = notarization_client
        .is_transfer_locked(*unlocked_notarization_id.object_id())
        .await?;

    println!("🔍 Transfer locked: {is_transfer_locked}");

    // Transfer the unlocked notarization to Alice
    let transfer_result = notarization_client
        .transfer_notarization(*unlocked_notarization_id.object_id(), alice)
        .build_and_execute(&notarization_client)
        .await;

    match transfer_result {
        Ok(_) => println!("✅ Successfully transferred unlocked notarization to Alice"),
        Err(e) => println!("❌ Failed to transfer: {e}"),
    }

    // Scenario 2: Try to transfer a transfer-locked dynamic notarization (should fail)
    println!("\n📝 Scenario 2: Creating a transfer-locked dynamic notarization...");

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 3600; // 1 hour

    let transfer_locked_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string("Transfer-locked document".to_string(), None))
        .with_immutable_description("Transfer-locked document".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created transfer-locked dynamic notarization: {transfer_locked_id:?}");

    let is_transfer_locked = notarization_client
        .is_transfer_locked(*transfer_locked_id.object_id())
        .await?;

    println!("🔍 Transfer locked: {is_transfer_locked}");

    // Try to transfer the locked notarization
    let transfer_result = notarization_client
        .transfer_notarization(*transfer_locked_id.object_id(), bob)
        .build_and_execute(&notarization_client)
        .await;

    match transfer_result {
        Ok(_) => println!("❌ Unexpected: Transfer succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: Transfer failed - {e}"),
    }

    // Scenario 3: Try to transfer a locked notarization (should always fail)
    println!("\n📝 Scenario 3: Creating a locked notarization...");

    let locked_notarization_id = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string("Locked document content".to_string(), None))
        .with_immutable_description("Locked document".to_string())
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created locked notarization: {locked_notarization_id:?}");

    let is_transfer_locked = notarization_client
        .is_transfer_locked(*locked_notarization_id.object_id())
        .await?;

    println!("🔍 Transfer locked: {is_transfer_locked}");

    // Try to transfer the locked notarization
    let transfer_result = notarization_client
        .transfer_notarization(*locked_notarization_id.object_id(), alice)
        .build_and_execute(&notarization_client)
        .await;

    match transfer_result {
        Ok(_) => println!("❌ Unexpected: Transfer succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: Transfer failed - {e}"),
    }

    // Show lock metadata for different scenarios
    println!("\n🔐 Lock Metadata Analysis:");

    let unlocked_lock_metadata = notarization_client
        .lock_metadata(*unlocked_notarization_id.object_id())
        .await?;

    let transfer_locked_lock_metadata = notarization_client
        .lock_metadata(*transfer_locked_id.object_id())
        .await?;

    let locked_lock_metadata = notarization_client
        .lock_metadata(*locked_notarization_id.object_id())
        .await?;

    println!("Unlocked notarization lock metadata: {unlocked_lock_metadata:?}");
    println!("Transfer-locked notarization lock metadata: {transfer_locked_lock_metadata:?}");
    println!("Locked notarization lock metadata: {locked_lock_metadata:?}");

    println!("\n📋 Transfer Rules Summary:");
    println!("✅ Unlocked dynamic notarizations can be transferred freely");
    println!("🔒 Transfer-locked dynamic notarizations cannot be transferred until lock expires");
    println!("🚫 Locked notarizations can never be transferred (transfer_lock = UntilDestroyed)");
    println!("⏰ Transfer locks are time-based and will expire automatically");
    println!("🔍 Use is_transfer_locked() to check transfer status before attempting");

    // Demonstrate checking multiple transfer statuses
    println!("\n🔍 Final Transfer Status Check:");

    let statuses = vec![
        ("Unlocked", unlocked_notarization_id),
        ("Transfer-locked", transfer_locked_id),
        ("Locked", locked_notarization_id),
    ];

    for (name, id) in statuses {
        let is_locked = notarization_client.is_transfer_locked(*id.object_id()).await?;
        println!("{name}: Transfer locked = {is_locked}");
    }

    Ok(())
}
