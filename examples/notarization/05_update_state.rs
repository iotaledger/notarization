// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::State;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating state updates on dynamic notarization");

    let notarization_client = get_funded_client().await?;

    println!("Creating a dynamic notarization for state updates...");

    // Create a dynamic notarization
    let notarization_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Initial document version".to_string(),
            Some("Version 1.0".to_string()),
        ))
        .with_immutable_description("Evolving document".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Created dynamic notarization: {notarization_id:?}");

    // Show initial state
    let initial_state = notarization_client.state(*notarization_id.object_id()).await?;

    let initial_version_count = notarization_client
        .state_version_count(*notarization_id.object_id())
        .await?;

    println!("\n📄 Initial State:");
    println!("Content: {}", initial_state.data.as_text()?);
    println!("Metadata: {:?}", initial_state.metadata);
    println!("Version count: {initial_version_count}");

    // Perform multiple state updates
    println!("\n🔄 Performing state updates...");

    for i in 1..=3 {
        println!("\n--- Update {i} ---");

        let new_state = State::from_string(
            format!("Updated document version {}", i + 1),
            Some(format!("Version {}.0 - Update {}", i + 1, i)),
        );

        // Update the state
        let _ = notarization_client
            .update_state(new_state.clone(), *notarization_id.object_id())
            .build_and_execute(&notarization_client)
            .await?;

        println!("✅ State update {i} completed");

        // Verify the update
        let current_state = notarization_client.state(*notarization_id.object_id()).await?;

        let version_count = notarization_client
            .state_version_count(*notarization_id.object_id())
            .await?;

        println!("Updated content: {}", current_state.data.as_text()?);
        println!("Updated metadata: {:?}", current_state.metadata);
        println!("New version count: {version_count}");

        // Verify version count incremented
        assert_eq!(version_count, i as u64);
    }

    // Show final version count
    let final_version_count = notarization_client
        .state_version_count(*notarization_id.object_id())
        .await?;

    let final_state = notarization_client.state(*notarization_id.object_id()).await?;

    println!("\n📊 Final Statistics:");
    println!("Total updates performed: {final_version_count}");
    println!("Final metadata: {:?}", final_state.metadata);

    // Get last state change timestamp
    let last_change = notarization_client
        .last_state_change_ts(*notarization_id.object_id())
        .await?;

    println!("Last state change timestamp: {last_change}");

    println!("\n🎯 Key Points:");
    println!("✓ Dynamic notarizations support state updates");
    println!("✓ Each update increments the version count");
    println!("✓ Both text and bytes data are supported");
    println!("✓ State metadata can be updated alongside content");
    println!("✓ Timestamps track when changes were made");

    Ok(())
}
