// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::{NotarizationMethod, State, TimeLock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Creating a dynamic notarization example");

    // Create a notarization client
    let notarization_client = get_funded_client().await?;

    println!("Creating a simple dynamic notarization without locks...");

    // Create a simple dynamic notarization
    let simple_notarization = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "Live document content".to_string(),
            Some("Version 1.0".to_string()),
        ))
        .with_immutable_description("Dynamic document".to_string())
        .with_updatable_metadata("Initial metadata".to_string())
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output;

    println!("✅ Simple dynamic notarization created!");
    println!("ID: {:?}", simple_notarization.id);

    // Calculate unlock time for transfer lock example
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 3600; // 1 hour

    println!("\nCreating a dynamic notarization with transfer lock...");

    // Create a dynamic notarization with transfer lock
    let locked_transfer_notarization = notarization_client
        .create_dynamic_notarization()
        .with_state(State::from_string("Transfer-locked content".to_string(), None))
        .with_immutable_description("Transfer-locked document".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&notarization_client)
        .await?
        .output;

    println!("✅ Transfer-locked dynamic notarization created!");
    println!("ID: {:?}", locked_transfer_notarization.id);

    // Verify both are dynamic notarizations
    assert_eq!(simple_notarization.method, NotarizationMethod::Dynamic);
    assert_eq!(locked_transfer_notarization.method, NotarizationMethod::Dynamic);

    // Check transfer lock status
    let is_transfer_locked = notarization_client
        .is_transfer_locked(*locked_transfer_notarization.id.object_id())
        .await?;

    println!("\n📊 Notarization Status:");
    println!("Simple notarization - Method: {:?}", simple_notarization.method);
    println!("Transfer-locked notarization - Transfer locked: {is_transfer_locked}");
    println!("\n🔄 Dynamic notarizations can be updated and transferred (unless transfer-locked)");
    println!("📝 State and metadata can be modified over time");

    Ok(())
}
