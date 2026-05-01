// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up the MaintenanceAdmin role.
//! - **Maintenance admin client**: Holds delete permissions. Attempts (and fails) to delete the non-empty trail, then
//!   batch-deletes all records before removing the trail itself.

use std::collections::HashSet;

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, Permission, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Show that a non-empty trail cannot be deleted.
/// 2. Empty the trail with `delete_records_batch`.
/// 3. Delete the trail once its records are gone.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Delete Trail ===\n");

    // Use a maintenance client to keep deletion permissions separate from trail creation.
    let admin_client = get_funded_audit_trail_client().await?;
    let maintenance_admin_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Initial record"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let maintenance_admin_role = "MaintenanceAdmin";
    let admin_trail = admin_client.trail(trail_id);

    // The Admin capability authorizes the maintenance role and capability delegation.
    admin_trail
        .access()
        .for_role(maintenance_admin_role)
        .create(
            PermissionSet {
                permissions: HashSet::from([Permission::DeleteAllRecords, Permission::DeleteAuditTrail]),
            },
            None,
        )
        .build_and_execute(&admin_client)
        .await?;
    admin_trail
        .access()
        .for_role(maintenance_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(maintenance_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    let maintenance_trail = maintenance_admin_client.trail(trail_id);

    let delete_while_non_empty = maintenance_trail
        .delete_audit_trail()
        .build_and_execute(&maintenance_admin_client)
        .await;
    ensure!(delete_while_non_empty.is_err(), "a trail must be empty before deletion");
    println!("Deleting the non-empty trail failed as expected.\n");

    // Batch delete skips locked records and returns the deleted sequence numbers before trail deletion.
    let deleted_records = maintenance_trail
        .records()
        .delete_records_batch(10)
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;
    println!("Deleted record sequence numbers {deleted_records:?} before trail removal.\n");

    ensure!(maintenance_trail.records().record_count().await? == 0);

    let deleted_trail = maintenance_trail
        .delete_audit_trail()
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;
    println!(
        "Trail deleted:\n  trail_id = {}\n  timestamp = {}",
        deleted_trail.trail_id, deleted_trail.timestamp
    );

    ensure!(
        maintenance_trail.get().await.is_err(),
        "deleted trail should no longer be readable"
    );

    Ok(())
}
