// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::state::State;
use notarization::core::timelock::TimeLock;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating update attempts on locked notarization");

    // Setup client (replace with your network configuration)
    let notarization_client = get_funded_client().await?;

    // Calculate unlock time
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // 24 hours

    println!("Creating a locked notarization...");

    // Create a locked notarization
    let locked_notarization_id = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string(
            "Original locked content".to_string(),
            Some("Original metadata".to_string()),
        ))
        .with_immutable_description("Locked document for update test".to_string())
        .with_updateable_metadata("Initial updateable metadata".to_string())
        .with_delete_at(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Locked notarization created with ID: {:?}", locked_notarization_id);

    // Check lock status
    let is_update_locked = notarization_client
        .is_update_locked(*locked_notarization_id.object_id())
        .await?;

    println!("🔒 Update locked status: {}", is_update_locked);

    // Attempt to update state (this should fail)
    println!("\n🔄 Attempting to update state on locked notarization...");
    let new_state = State::from_string(
        "Attempted updated content".to_string(),
        Some("Attempted new metadata".to_string()),
    );

    let state_update_result = notarization_client
        .update_state(new_state, *locked_notarization_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match state_update_result {
        Ok(_) => println!("❌ Unexpected: State update succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: State update failed - {}", e),
    }

    // Attempt to update metadata (this should also fail)
    println!("\n📝 Attempting to update metadata on locked notarization...");
    let new_metadata = Some("Attempted updated metadata".to_string());

    let metadata_update_result = notarization_client
        .update_metadata(new_metadata, *locked_notarization_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match metadata_update_result {
        Ok(_) => println!("❌ Unexpected: Metadata update succeeded (should have failed)"),
        Err(e) => println!("✅ Expected: Metadata update failed - {}", e),
    }

    // Show current state is unchanged
    println!("\n📊 Verifying original state is preserved...");
    let current_state = notarization_client.state(*locked_notarization_id.object_id()).await?;

    println!("Current state: {}", current_state.data.as_text()?);
    println!("Current state metadata: {:?}", current_state.metadata);

    let current_updateable_metadata = notarization_client
        .updateable_metadata(*locked_notarization_id.object_id())
        .await?;

    println!("Current updateable metadata: {:?}", current_updateable_metadata);

    println!("\n🔒 Locked notarizations are immutable - state and metadata cannot be changed");
    println!("📅 They can only be destroyed after the delete lock expires");

    Ok(())
}
