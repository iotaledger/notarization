// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up the RecordAdmin role.
//! - **RecordAdmin**: Adds one follow-up record. All subsequent operations are read-only
//!   and can be performed by any address — no capability required.

use anyhow::{Result, ensure};
use audit_trail::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, LockingConfig, LockingWindow, PermissionSet,
    TimeLock,
};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Load the full on-chain trail object.
/// 2. Inspect metadata, roles, and locking configuration.
/// 3. Read records individually and through pagination.
/// 4. Query the record-count and lock-status helpers.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Read-Only Inspection ===\n");

    // `admin` creates the trail and manages roles.
    // `record_admin` adds the follow-up record.
    let admin = get_funded_audit_trail_client().await?;
    let record_admin = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_trail_metadata(ImmutableMetadata::new(
            "Operations Trail".to_string(),
            Some("Used to inspect read-only accessors".to_string()),
        ))
        .with_updatable_metadata("Status: Active")
        .with_locking_config(LockingConfig {
            delete_record_window: LockingWindow::CountBased { count: 2 },
            delete_trail_lock: TimeLock::None,
            write_lock: TimeLock::None,
        })
        .with_initial_record(InitialRecord::new(
            Data::text("Initial record"),
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
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    record_admin
        .trail(trail_id)
        .records()
        .add(Data::text("Follow-up record"), Some("event:updated".to_string()), None)
        .build_and_execute(&record_admin)
        .await?;

    let on_chain = admin.trail(trail_id).get().await?;
    println!(
        "Trail summary:\n  id = {}\n  creator = {}\n  created_at = {}\n  sequence_number = {}\n  immutable_metadata = {:?}\n  updatable_metadata = {:?}\n",
        on_chain.id.object_id(),
        on_chain.creator,
        on_chain.created_at,
        on_chain.sequence_number,
        on_chain.immutable_metadata,
        on_chain.updatable_metadata
    );

    println!(
        "Roles: {:?}\nLocking config: {:?}\n",
        on_chain.roles.roles.keys().collect::<Vec<_>>(),
        on_chain.locking_config
    );

    let trail = admin.trail(trail_id);
    let count = trail.records().record_count().await?;
    let initial_record = trail.records().get(0).await?;
    let first_page = trail.records().list_page(None, 10).await?;
    let record_zero_locked = trail.locking().is_record_locked(0).await?;

    println!("Record count: {count}");
    println!("Record #0: {:?}", initial_record);
    println!(
        "First page size: {} (has_next_page = {})",
        first_page.records.len(),
        first_page.has_next_page
    );
    println!("Is record #0 locked? {record_zero_locked}");

    ensure!(count == 2);
    ensure!(matches!(initial_record.data, Data::Text(ref text) if text == "Initial record"));
    ensure!(first_page.records.len() == 2);

    Ok(())
}
