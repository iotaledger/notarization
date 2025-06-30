// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::{NotarizationMethod, State, TimeLock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Creating a locked notarization example");

    // Create a notarization client
    let notarization_client = get_funded_client().await?;

    // Calculate unlock time (24 hours from now)
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // 24 hours

    println!("Creating locked notarization with delete lock until: {unlock_at}");

    // Create a locked notarization with state and delete lock
    let locked_notarization = notarization_client
        .create_locked_notarization()
        .with_state(State::from_string(
            "Important document content".to_string(),
            Some("Document metadata".to_string()),
        ))
        .with_immutable_description("Critical legal document".to_string())
        .with_updatable_metadata("Initial document metadata".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&notarization_client)
        .await?
        .output;

    println!("✅ Locked notarization created successfully!");
    println!("Notarization ID: {:?}", locked_notarization.id);
    println!("Method: {:?}", locked_notarization.method);
    println!("Description: {:?}", locked_notarization.immutable_metadata.description);
    println!("Updatable metadata: {:?}", locked_notarization.updatable_metadata);
    println!("State version count: {}", locked_notarization.state_version_count);

    // Verify the notarization method is locked
    assert_eq!(locked_notarization.method, NotarizationMethod::Locked);

    // Check if it has locking metadata
    assert!(locked_notarization.immutable_metadata.locking.is_some());

    println!("🔒 The notarization is locked and cannot be updated or transferred until destroyed");
    println!("🗑️ The notarization can only be destroyed after the delete lock expires");

    Ok(())
}
