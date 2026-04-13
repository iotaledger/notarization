// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up the MaintenanceAdmin role.
//! - **MaintenanceAdmin**: Holds delete permissions. Attempts (and fails) to delete the non-empty trail, then
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

    // `admin` creates the trail and manages roles.
    // `maintenance_admin` empties and deletes the trail.
    let admin = get_funded_audit_trail_client().await?;
    let maintenance_admin = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Initial record"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail = admin.trail(created.trail_id);

    trail
        .access()
        .for_role("MaintenanceAdmin")
        .create(
            PermissionSet {
                permissions: HashSet::from([Permission::DeleteAllRecords, Permission::DeleteAuditTrail]),
            },
            None,
        )
        .build_and_execute(&admin)
        .await?;
    trail
        .access()
        .for_role("MaintenanceAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(maintenance_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    let maintenance_trail = maintenance_admin.trail(created.trail_id);

    let delete_while_non_empty = maintenance_trail
        .delete_audit_trail()
        .build_and_execute(&maintenance_admin)
        .await;
    ensure!(delete_while_non_empty.is_err(), "a trail must be empty before deletion");
    println!("Deleting the non-empty trail failed as expected.\n");

    let deleted_records = maintenance_trail
        .records()
        .delete_records_batch(10)
        .build_and_execute(&maintenance_admin)
        .await?
        .output;
    println!("Deleted {deleted_records} record(s) before trail removal.\n");

    ensure!(maintenance_trail.records().record_count().await? == 0);

    let deleted_trail = maintenance_trail
        .delete_audit_trail()
        .build_and_execute(&maintenance_admin)
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
