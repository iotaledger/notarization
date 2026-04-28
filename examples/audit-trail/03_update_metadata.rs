// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up the MetadataAdmin role.
//! - **Metadata admin client**: Holds the MetadataAdmin capability and updates the trail's mutable status field. Has no
//!   record-write permissions.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create a trail with immutable and updatable metadata.
/// 2. Delegate metadata updates through a dedicated `MetadataAdmin` role.
/// 3. Change and clear the trail's updatable metadata.
/// 4. Verify that immutable metadata never changes.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Update Metadata ===\n");

    // Use separate clients so metadata updates are clearly delegated away from the creator.
    let admin_client = get_funded_audit_trail_client().await?;
    let metadata_admin_client = get_funded_audit_trail_client().await?;

    let immutable_metadata = ImmutableMetadata::new(
        "Shipment Processing".to_string(),
        Some("Tracks the lifecycle of a warehouse shipment".to_string()),
    );

    let created_trail = admin_client
        .create_trail()
        .with_trail_metadata(immutable_metadata.clone())
        .with_updatable_metadata("Status: Draft")
        .with_initial_record(InitialRecord::new(
            Data::text("Shipment created"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let metadata_admin_role = "MetadataAdmin";

    // The Admin capability in `admin_client`'s wallet authorizes role definition and capability issuance.
    admin_client
        .trail(trail_id)
        .access()
        .for_role(metadata_admin_role)
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;

    admin_client
        .trail(trail_id)
        .access()
        .for_role(metadata_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(metadata_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    let trail_before_update = admin_client.trail(trail_id).get().await?;
    println!(
        "Before update:\n  immutable = {:?}\n  updatable = {:?}\n",
        trail_before_update.immutable_metadata, trail_before_update.updatable_metadata
    );

    metadata_admin_client
        .trail(trail_id)
        .update_metadata(Some("Status: In Review".to_string()))
        .build_and_execute(&metadata_admin_client)
        .await?;

    let trail_after_update = admin_client.trail(trail_id).get().await?;
    println!(
        "After update:\n  immutable = {:?}\n  updatable = {:?}\n",
        trail_after_update.immutable_metadata, trail_after_update.updatable_metadata
    );

    ensure!(trail_after_update.immutable_metadata == Some(immutable_metadata.clone()));
    ensure!(trail_after_update.updatable_metadata.as_deref() == Some("Status: In Review"));

    metadata_admin_client
        .trail(trail_id)
        .update_metadata(None)
        .build_and_execute(&metadata_admin_client)
        .await?;

    let trail_after_clear = admin_client.trail(trail_id).get().await?;
    println!(
        "After clear:\n  immutable = {:?}\n  updatable = {:?}",
        trail_after_clear.immutable_metadata, trail_after_clear.updatable_metadata
    );

    ensure!(trail_after_clear.immutable_metadata == Some(immutable_metadata));
    ensure!(trail_after_clear.updatable_metadata.is_none());

    Ok(())
}
