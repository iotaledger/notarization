// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::State;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Demonstrating update on dynamic notarization");

    let notarization_client = get_funded_client().await?;

    println!("Creating a dynamic notarization...");

    // Create a dynamic notarization
    let dynamic_notarization_id = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Original content".to_string(),
            Some("Original metadata".to_string()),
        ))
        .with_immutable_description("Dynamic document for update test".to_string())
        .with_updatable_metadata("Initial updatable metadata".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output
        .id;

    println!("✅ Dynamic notarization created with ID: {dynamic_notarization_id:?}");

    let current_state = notarization_client.state(*dynamic_notarization_id.object_id()).await?;
    println!("Initial state: {}", current_state.data.as_text()?);
    println!("Initial state metadata: {:?}", current_state.metadata);

    println!("\n🔄 Updating state on dynamic notarization...");
    let new_state = State::from_string("Updated content".to_string(), Some("New metadata".to_string()));

    let state_update_result = notarization_client
        .update_state(new_state, *dynamic_notarization_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match state_update_result {
        Ok(_) => println!("✅ State update succeeded"),
        Err(e) => println!("❌ State update failed - {e}"),
    }

    println!("\n📝 Updating metadata on dynamic notarization...");
    let new_metadata = Some("Updated metadata".to_string());

    let metadata_update_result = notarization_client
        .update_metadata(new_metadata, *dynamic_notarization_id.object_id())
        .build_and_execute(&notarization_client)
        .await;

    match metadata_update_result {
        Ok(_) => println!("✅ Metadata update succeeded"),
        Err(e) => println!("❌ Metadata update failed - {e}"),
    }

    let current_state = notarization_client.state(*dynamic_notarization_id.object_id()).await?;
    println!("Updated state: {}", current_state.data.as_text()?);
    println!("Updated state metadata: {:?}", current_state.metadata);

    println!("\n🔒 Dynamic notarizations are mutable - state and metadata can be changed");

    Ok(())
}
