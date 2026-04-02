// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_notarization_client;
use notarization::core::types::{State, TimeLock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating notarization destruction scenarios");

    // Create a notarization client
    let notarization_client = get_funded_notarization_client().await?;

    // Scenario 1: Destroy an unlocked dynamic notarization (should succeed)
    println!("📝 Scenario 1: Creating and destroying an unlocked dynamic notarization...");

    let unlocked_dynamic_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string("Unlocked content".to_string(), None))
        .with_immutable_description("Unlocked dynamic document".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created unlocked dynamic notarization: {unlocked_dynamic_id:?}");

    // Check if destroy is allowed
    let is_destroy_allowed = notarization_client
        .is_destroy_allowed(*unlocked_dynamic_id.object_id())
        .await?;

    println!("🔍 Destroy allowed: {is_destroy_allowed}");

    // Destroy the unlocked notarization
    let destroy_result = notarization_client
        .destroy(*unlocked_dynamic_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match destroy_result {
        Ok(_) => println!("✅ Successfully destroyed unlocked dynamic notarization"),
        Err(e) => println!("❌ Failed to destroy: {e}"),
    }

    // Scenario 2: Try to destroy a transfer-locked dynamic notarization (should fail)
    println!("\n📝 Scenario 2: Creating a transfer-locked dynamic notarization...");

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // 24 hours

    let transfer_locked_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string("Transfer-locked content".to_string(), None))
        .with_immutable_description("Transfer-locked document".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created transfer-locked dynamic notarization: {transfer_locked_id:?}");

    let is_destroy_allowed = notarization_client
        .is_destroy_allowed(*transfer_locked_id.object_id())
        .await?;

    println!("🔍 Destroy allowed: {is_destroy_allowed}");

    // Try to destroy the transfer-locked notarization
    let destroy_result = notarization_client
        .destroy(*transfer_locked_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match destroy_result {
        Ok(_) => println!("❌ Unexpected: Destruction succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: Destruction failed - {e}"),
    }

    // Scenario 3: Create and try to destroy a time-locked locked notarization (should fail)
    println!("\n📝 Scenario 3: Creating a time-locked locked notarization...");

    let delete_locked_id = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string("Delete-locked content".to_string(), None))
        .with_immutable_description("Delete-locked document".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created delete-locked locked notarization: {delete_locked_id:?}");

    let is_destroy_allowed = notarization_client
        .is_destroy_allowed(*delete_locked_id.object_id())
        .await?;

    println!("🔍 Destroy allowed: {is_destroy_allowed}");

    // Try to destroy the delete-locked notarization
    let destroy_result = notarization_client
        .destroy(*delete_locked_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match destroy_result {
        Ok(_) => println!("❌ Unexpected: Destruction succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: Destruction failed - {e}"),
    }

    // Scenario 4: Create and destroy a locked notarization with no delete lock (should succeed)
    println!("\n📝 Scenario 4: Creating a locked notarization with no delete lock...");

    let no_delete_lock_id = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string("No delete lock content".to_string(), None))
        .with_immutable_description("No delete lock document".to_string())
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created locked notarization with no delete lock: {no_delete_lock_id:?}");

    let is_destroy_allowed = notarization_client
        .is_destroy_allowed(*no_delete_lock_id.object_id())
        .await?;

    println!("🔍 Destroy allowed: {is_destroy_allowed}");

    // Destroy the notarization with no delete lock
    let destroy_result = notarization_client
        .destroy(*no_delete_lock_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match destroy_result {
        Ok(_) => println!("✅ Successfully destroyed locked notarization with no delete lock"),
        Err(e) => println!("❌ Failed to destroy: {e}"),
    }

    println!("\n📋 Summary:");
    println!("🔓 Unlocked notarizations can be destroyed immediately");
    println!("🔒 Transfer-locked dynamic notarizations cannot be destroyed");
    println!("⏰ Time-locked locked notarizations cannot be destroyed before lock expires");
    println!("🆓 Locked notarizations with TimeLock::None can be destroyed");

    Ok(())
}
