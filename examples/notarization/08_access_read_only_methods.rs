// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_notarization_client;
use notarization::core::types::{State, TimeLock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating read-only methods for notarization inspection");

    let notarization_client = get_funded_notarization_client().await?;

    // Create a comprehensive dynamic notarization for testing
    println!("Creating a dynamic notarization with comprehensive metadata...");

    let description = "Comprehensive test document".to_string();
    let updatable_metadata = "Initial document metadata".to_string();

    let dynamic_notarization_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Document content with detailed metadata".to_string(),
            Some("State-level metadata".to_string()),
        ))
        .with_immutable_description(description.clone())
        .with_updatable_metadata(updatable_metadata.clone())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created dynamic notarization: {dynamic_notarization_id:?}");

    // Demonstrate all read-only methods for dynamic notarization
    println!("\n📖 Read-only Methods for Dynamic Notarization:");

    // 1. Get description (immutable)
    let retrieved_description = notarization_client
        .description(*dynamic_notarization_id.object_id())
        .await?;
    println!("📝 Description: {retrieved_description:?}");

    // 2. Get updateable metadata
    let retrieved_metadata = notarization_client
        .updatable_metadata(*dynamic_notarization_id.object_id())
        .await?;
    println!("📋 Updatable metadata: {retrieved_metadata:?}");

    // 3. Get current state
    let current_state = notarization_client.state(*dynamic_notarization_id.object_id()).await?;
    println!("📄 State content: {}", current_state.data.as_text()?);
    println!("📄 State metadata: {:?}", current_state.metadata);

    // 4. Get creation timestamp
    let created_at = notarization_client
        .created_at_ts(*dynamic_notarization_id.object_id())
        .await?;
    println!("🕐 Created at timestamp: {created_at}");

    // 5. Get last state change timestamp
    let last_state_change = notarization_client
        .last_state_change_ts(*dynamic_notarization_id.object_id())
        .await?;
    println!("🕐 Last state change timestamp: {last_state_change}");

    // 6. Get state version count
    let version_count = notarization_client
        .state_version_count(*dynamic_notarization_id.object_id())
        .await?;
    println!("🔢 State version count: {version_count}");

    // 7. Get notarization method
    let method = notarization_client
        .notarization_method(*dynamic_notarization_id.object_id())
        .await?;
    println!("⚙️ Notarization method: {method:?}");

    // 8. Check lock statuses
    let is_transfer_locked = notarization_client
        .is_transfer_locked(*dynamic_notarization_id.object_id())
        .await?;
    let is_update_locked = notarization_client
        .is_update_locked(*dynamic_notarization_id.object_id())
        .await?;
    let is_destroy_allowed = notarization_client
        .is_destroy_allowed(*dynamic_notarization_id.object_id())
        .await?;
    println!("🔒 Transfer locked: {is_transfer_locked}");
    println!("🔒 Update locked: {is_update_locked}");
    println!("🗑️ Destroy allowed: {is_destroy_allowed}");

    // 9. Get lock metadata
    let lock_metadata = notarization_client
        .lock_metadata(*dynamic_notarization_id.object_id())
        .await?;
    println!("🔐 Lock metadata: {lock_metadata:?}");

    // 10. Get the whole OnChainNotarization at once and pretty print it
    let on_chain_notarization = notarization_client
        .get_notarization_by_id(*dynamic_notarization_id.object_id())
        .await?;
    println!("📦 Complete dynamic OnChainNotarization:\n{on_chain_notarization:#?}");

    // Update the state to demonstrate version tracking
    println!("\n🔄 Updating state to demonstrate version tracking...");

    let new_state = State::from_string(
        "Updated document content".to_string(),
        Some("Updated state metadata".to_string()),
    );

    notarization_client
        .update_state(new_state, *dynamic_notarization_id.object_id())
        .build_and_execute(&notarization_client)
        .await?;

    // Show updated read-only values
    println!("\n📊 After State Update:");

    let updated_version_count = notarization_client
        .state_version_count(*dynamic_notarization_id.object_id())
        .await?;
    let updated_last_change = notarization_client
        .last_state_change_ts(*dynamic_notarization_id.object_id())
        .await?;
    let updated_state = notarization_client.state(*dynamic_notarization_id.object_id()).await?;

    println!("🔢 New version count: {updated_version_count}");
    println!("🕐 Updated last change timestamp: {updated_last_change}");
    println!("📄 Updated state content: {}", updated_state.data.as_text()?);

    // Create a locked notarization for comparison
    println!("\n🔒 Creating a locked notarization for comparison...");

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // 24 hours

    let locked_notarization_id = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string(
            "Locked document content".to_string(),
            Some("Locked state metadata".to_string()),
        ))
        .with_immutable_description("Locked test document".to_string())
        .with_updatable_metadata("Locked document metadata".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created locked notarization: {locked_notarization_id:?}");

    // Demonstrate read-only methods for locked notarization
    println!("\n📖 Read-only Methods for Locked Notarization:");

    let locked_method = notarization_client
        .notarization_method(*locked_notarization_id.object_id())
        .await?;
    let locked_transfer_locked = notarization_client
        .is_transfer_locked(*locked_notarization_id.object_id())
        .await?;
    let locked_update_locked = notarization_client
        .is_update_locked(*locked_notarization_id.object_id())
        .await?;
    let locked_destroy_allowed = notarization_client
        .is_destroy_allowed(*locked_notarization_id.object_id())
        .await?;
    let locked_lock_metadata = notarization_client
        .lock_metadata(*locked_notarization_id.object_id())
        .await?;
    let whole_locked_notarization = notarization_client
        .get_notarization_by_id(*locked_notarization_id.object_id())
        .await?;

    println!("⚙️ Method: {locked_method:?}");
    println!("🔒 Transfer locked: {locked_transfer_locked}");
    println!("🔒 Update locked: {locked_update_locked}");
    println!("🗑️ Destroy allowed: {locked_destroy_allowed}");
    println!("🔐 Lock metadata present: {}", locked_lock_metadata.is_some());
    println!("📦 Complete locked OnChainNotarization:\n{whole_locked_notarization:#?}");

    // Compare methods between dynamic and locked
    println!("\n📊 Comparison Summary:");
    println!("┌─────────────────────┬─────────────┬─────────────┐");
    println!("│ Property            │ Dynamic     │ Locked      │");
    println!("├─────────────────────┼─────────────┼─────────────┤");
    println!(
        "│ Method              │ {:11} │ {:11} │",
        format!("{:?}", method),
        format!("{:?}", locked_method)
    );
    println!("│ Transfer Locked     │ {is_transfer_locked:11} │ {locked_transfer_locked:11} │");
    println!("│ Update Locked       │ {is_update_locked:11} │ {locked_update_locked:11} │");
    println!("│ Destroy Allowed     │ {is_destroy_allowed:11} │ {locked_destroy_allowed:11} │");
    println!(
        "│ Has Lock Metadata   │ {:11} │ {:11} │",
        lock_metadata.is_some(),
        locked_lock_metadata.is_some()
    );
    println!(
        "│ Owner               │ {:11} │ {:11} │",
        on_chain_notarization.owner, whole_locked_notarization.owner
    );
    println!("└─────────────────────┴─────────────┴─────────────┘");

    println!("\n🎯 Key Points about Read-only Methods:");
    println!("✓ All notarizations support the same read-only interface");
    println!("✓ State version count tracks state updates (not metadata updates)");
    println!("✓ Timestamps help track creation and modification times");
    println!("✓ Lock checking methods help determine allowed operations");
    println!("✓ Dynamic and locked notarizations have different lock behaviors");
    println!("✓ Lock metadata provides detailed information about applied locks");

    Ok(())
}
