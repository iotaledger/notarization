// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, Permission, PermissionSet};
use examples::get_funded_audit_trail_client;

/// Demonstrates how to:
/// 1. Show that a non-empty trail cannot be deleted.
/// 2. Empty the trail with `delete_records_batch`.
/// 3. Delete the trail once its records are gone.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Delete Trail ===\n");

    let client = get_funded_audit_trail_client().await?;

    let created = client
        .create_trail()
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
        .for_role("MaintenanceAdmin")
        .create(
            PermissionSet {
                permissions: HashSet::from([Permission::DeleteAllRecords, Permission::DeleteAuditTrail]),
            },
            None,
        )
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("MaintenanceAdmin")
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?;

    let delete_while_non_empty = trail.delete_audit_trail().build_and_execute(&client).await;
    ensure!(delete_while_non_empty.is_err(), "a trail must be empty before deletion");
    println!("Deleting the non-empty trail failed as expected.\n");

    let deleted_records = trail
        .records()
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await?
        .output;
    println!("Deleted {deleted_records} record(s) before trail removal.\n");

    ensure!(trail.records().record_count().await? == 0);

    let deleted_trail = trail.delete_audit_trail().build_and_execute(&client).await?.output;
    println!(
        "Trail deleted:\n  trail_id = {}\n  timestamp = {}",
        deleted_trail.trail_id, deleted_trail.timestamp
    );

    ensure!(trail.get().await.is_err(), "deleted trail should no longer be readable");

    Ok(())
}
