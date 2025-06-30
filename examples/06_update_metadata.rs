// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::State;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating metadata updates on dynamic notarization");

    let notarization_client = get_funded_client().await?;

    println!("Creating a dynamic notarization for metadata updates...");

    // Create a dynamic notarization with initial updateable metadata
    let notarization_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Document with evolving metadata".to_string(),
            Some("State metadata".to_string()),
        ))
        .with_immutable_description("Document for metadata testing".to_string())
        .with_updatable_metadata("Initial document metadata".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created dynamic notarization: {notarization_id:?}");

    // Show initial metadata
    let initial_updatable_metadata = notarization_client
        .updatable_metadata(*notarization_id.object_id())
        .await?;

    let initial_description = notarization_client.description(*notarization_id.object_id()).await?;

    let initial_version_count = notarization_client
        .state_version_count(*notarization_id.object_id())
        .await?;

    println!("\n📄 Initial Metadata:");
    println!("Immutable description: {initial_description:?}");
    println!("Updatable metadata: {initial_updatable_metadata:?}");
    println!("State version count: {initial_version_count}");

    // Update metadata multiple times
    println!("\n🔄 Performing metadata updates...");

    let metadata_updates = [
        Some("Updated metadata - Phase 1".to_string()),
        Some("Updated metadata - Phase 2".to_string()),
        Some("Updated metadata - Phase 3".to_string()),
        None, // Clear metadata
        Some("Final metadata".to_string()),
    ];

    for (i, new_metadata) in metadata_updates.iter().enumerate() {
        println!("\n--- Metadata Update {} ---", i + 1);

        // Update the metadata
        let _ = notarization_client
            .update_metadata(new_metadata.clone(), *notarization_id.object_id())
            .build_and_execute(&notarization_client)
            .await?;

        println!("✅ Metadata update {} completed", i + 1);

        // Verify the update
        let current_metadata = notarization_client
            .updatable_metadata(*notarization_id.object_id())
            .await?;

        let version_count = notarization_client
            .state_version_count(*notarization_id.object_id())
            .await?;

        println!("Updated metadata: {current_metadata:?}");
        println!("Version count (should remain unchanged): {version_count}");

        // Verify that state version count doesn't change with metadata updates
        assert_eq!(version_count, initial_version_count);

        // Verify metadata matches what we set
        assert_eq!(current_metadata, *new_metadata);
    }

    // Demonstrate that immutable description cannot be changed
    println!("\n📋 Verifying immutable description remains unchanged...");

    let final_description = notarization_client.description(*notarization_id.object_id()).await?;

    println!("Final description: {final_description:?}");
    assert_eq!(final_description, initial_description);

    // Show that state content is unaffected by metadata updates
    println!("\n📄 Verifying state content is unaffected...");

    let final_state = notarization_client.state(*notarization_id.object_id()).await?;

    println!("State content: {}", final_state.data.as_text()?);
    println!("State metadata: {:?}", final_state.metadata);

    // Get timestamps to show metadata updates don't affect state change time
    let created_at = notarization_client.created_at_ts(*notarization_id.object_id()).await?;

    let last_state_change = notarization_client
        .last_state_change_ts(*notarization_id.object_id())
        .await?;

    println!("\n⏰ Timestamps:");
    println!("Created at: {created_at}");
    println!("Last state change: {last_state_change}");

    // Final metadata state
    let final_updatable_metadata = notarization_client
        .updatable_metadata(*notarization_id.object_id())
        .await?;

    println!("\n📊 Final State:");
    println!("Final updatable metadata: {final_updatable_metadata:?}");
    println!(
        "State version count: {}",
        notarization_client
            .state_version_count(*notarization_id.object_id())
            .await?
    );

    println!("\n🎯 Key Points:");
    println!("✓ Updatable metadata can be changed on dynamic notarizations");
    println!("✓ Metadata updates don't affect state version count");
    println!("✓ Metadata can be set to None (cleared)");
    println!("✓ Immutable description cannot be changed after creation");
    println!("✓ State content is unaffected by metadata updates");
    println!("✓ Last state change timestamp is not updated by metadata changes");

    Ok(())
}
