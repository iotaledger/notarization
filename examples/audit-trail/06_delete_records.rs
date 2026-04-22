// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up the RecordMaintenance role.
//! - **RecordMaintainer**: Holds the RecordMaintenance capability. Adds records and then deletes them individually and
//!   in batch.

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

    // `admin` creates the trail and manages roles.
    // `record_maintainer` adds and deletes records.
    let admin = get_funded_audit_trail_client().await?;
    let record_maintainer = get_funded_audit_trail_client().await?;

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
        .for_role("RecordMaintenance")
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
        .build_and_execute(&admin)
        .await?;

    trail
        .access()
        .for_role("RecordMaintenance")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_maintainer.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    let records = record_maintainer.trail(created.trail_id).records();

    let added_one = records
        .add(Data::text("Second record"), Some("event:received".to_string()), None)
        .build_and_execute(&record_maintainer)
        .await?
        .output;
    let added_two = records
        .add(Data::text("Third record"), Some("event:dispatched".to_string()), None)
        .build_and_execute(&record_maintainer)
        .await?
        .output;

    println!(
        "Trail has records at sequence numbers 0, {}, {}\n",
        added_one.sequence_number, added_two.sequence_number
    );
    ensure!(records.record_count().await? == 3);

    let deleted_one = records
        .delete(added_one.sequence_number)
        .build_and_execute(&record_maintainer)
        .await?
        .output;
    println!("Deleted record {}\n", deleted_one.sequence_number);

    ensure!(records.record_count().await? == 2);
    ensure!(
        records.get(added_one.sequence_number).await.is_err(),
        "deleted record should no longer be readable"
    );

    let deleted_remaining = records
        .delete_records_batch(10)
        .build_and_execute(&record_maintainer)
        .await?
        .output;

    println!("Batch deleted the remaining sequence numbers: {deleted_remaining:?}");
    ensure!(deleted_remaining == vec![1, 2]);
    ensure!(records.record_count().await? == 0);

    Ok(())
}
