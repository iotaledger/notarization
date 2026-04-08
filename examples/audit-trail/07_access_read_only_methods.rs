// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, ensure};
use audit_trail::core::types::{
    Data, ImmutableMetadata, InitialRecord, LockingConfig, LockingWindow, PermissionSet, TimeLock,
};
use examples::get_funded_audit_trail_client;

/// Demonstrates how to:
/// 1. Load the full on-chain trail object.
/// 2. Inspect metadata, roles, and locking configuration.
/// 3. Read records individually and through pagination.
/// 4. Query the record-count and lock-status helpers.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Read-Only Inspection ===\n");

    let client = get_funded_audit_trail_client().await?;

    let created = client
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
        .build_and_execute(&client)
        .await?
        .output;

    let trail = client.trail(created.trail_id);

    trail
        .access()
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("RecordAdmin")
        .issue_capability(Default::default())
        .build_and_execute(&client)
        .await?;

    trail
        .records()
        .add(Data::text("Follow-up record"), Some("event:updated".to_string()), None)
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
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
