// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up the MetadataAdmin role.
//! - **MetadataAdmin**: Holds the MetadataAdmin capability and updates the trail's mutable status field. Has no
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

    // `admin` creates the trail and manages roles.
    // `metadata_admin` holds the MetadataAdmin capability and updates the trail status.
    let admin = get_funded_audit_trail_client().await?;
    let metadata_admin = get_funded_audit_trail_client().await?;

    let immutable_metadata = ImmutableMetadata::new(
        "Shipment Processing".to_string(),
        Some("Tracks the lifecycle of a warehouse shipment".to_string()),
    );

    let created = admin
        .create_trail()
        .with_trail_metadata(immutable_metadata.clone())
        .with_updatable_metadata("Status: Draft")
        .with_initial_record(InitialRecord::new(
            Data::text("Shipment created"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;

    admin
        .trail(trail_id)
        .access()
        .for_role("MetadataAdmin")
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;

    admin
        .trail(trail_id)
        .access()
        .for_role("MetadataAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(metadata_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    let before = admin.trail(trail_id).get().await?;
    println!(
        "Before update:\n  immutable = {:?}\n  updatable = {:?}\n",
        before.immutable_metadata, before.updatable_metadata
    );

    metadata_admin
        .trail(trail_id)
        .update_metadata(Some("Status: In Review".to_string()))
        .build_and_execute(&metadata_admin)
        .await?;

    let after_update = admin.trail(trail_id).get().await?;
    println!(
        "After update:\n  immutable = {:?}\n  updatable = {:?}\n",
        after_update.immutable_metadata, after_update.updatable_metadata
    );

    ensure!(after_update.immutable_metadata == Some(immutable_metadata.clone()));
    ensure!(after_update.updatable_metadata.as_deref() == Some("Status: In Review"));

    metadata_admin
        .trail(trail_id)
        .update_metadata(None)
        .build_and_execute(&metadata_admin)
        .await?;

    let after_clear = admin.trail(trail_id).get().await?;
    println!(
        "After clear:\n  immutable = {:?}\n  updatable = {:?}",
        after_clear.immutable_metadata, after_clear.updatable_metadata
    );

    ensure!(after_clear.immutable_metadata == Some(immutable_metadata));
    ensure!(after_clear.updatable_metadata.is_none());

    Ok(())
}
