// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up the RecordMaintenance role.
//! - **Maintenance admin client**: Holds the RecordMaintenance capability. Adds records and then deletes them
//!   individually and in batch.

use std::collections::HashSet;

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, Permission, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create records using a delegated record-maintenance role.
/// 2. Delete a single record by sequence number.
/// 3. Delete the remaining records in one batch.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Delete Records ===\n");

    // Use a maintenance client to show deletes happening through a delegated capability.
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
    let maintenance_admin_role = "RecordMaintenance";
    let admin_trail = admin_client.trail(trail_id);

    // This role grants both single-record and batch-delete permissions.
    admin_trail
        .access()
        .for_role(maintenance_admin_role)
        .create(
            PermissionSet {
                permissions: HashSet::from([
                    Permission::AddRecord,
                    Permission::DeleteRecord,
                    Permission::DeleteAllRecords,
                ]),
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

    let maintenance_records = maintenance_admin_client.trail(trail_id).records();

    let first_added_record = maintenance_records
        .add(Data::text("Second record"), Some("event:received".to_string()), None)
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;
    let second_added_record = maintenance_records
        .add(Data::text("Third record"), Some("event:dispatched".to_string()), None)
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;

    println!(
        "Trail has records at sequence numbers 0, {}, {}\n",
        first_added_record.sequence_number, second_added_record.sequence_number
    );
    ensure!(maintenance_records.record_count().await? == 3);

    let deleted_record = maintenance_records
        .delete(first_added_record.sequence_number)
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;
    println!("Deleted record {}\n", deleted_record.sequence_number);

    ensure!(maintenance_records.record_count().await? == 2);
    ensure!(
        maintenance_records
            .get(first_added_record.sequence_number)
            .await
            .is_err(),
        "deleted record should no longer be readable"
    );

    // Batch delete skips locked records and returns the deleted sequence numbers.
    let deleted_remaining = maintenance_records
        .delete_records_batch(10)
        .build_and_execute(&maintenance_admin_client)
        .await?
        .output;

    println!("Batch deleted the remaining sequence numbers: {deleted_remaining:?}");
    ensure!(deleted_remaining == vec![0, second_added_record.sequence_number]);
    ensure!(maintenance_records.record_count().await? == 0);

    Ok(())
}
