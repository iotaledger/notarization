// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail, defines the RecordAdmin role, and issues a capability.
//! - **RecordAdmin**: Holds the capability and writes records. Reads are also done through
//!   this client to demonstrate that any address can read, but only the cap holder can write.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create an audit trail with an initial record.
/// 2. Define a `RecordAdmin` role and issue a capability for it.
/// 3. Add follow-up records to the trail.
/// 4. Read records back individually and through paginated traversal.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Add & Read Records ===\n");

    // `admin` creates the trail and manages roles.
    // `record_admin` holds the RecordAdmin capability and writes records.
    let admin = get_funded_audit_trail_client().await?;
    let record_admin = get_funded_audit_trail_client().await?;

    println!("Admin address:        {}", admin.sender_address());
    println!("RecordAdmin address:  {}\n", record_admin.sender_address());

    // -------------------------------------------------------------------------
    // Step 1: Create a trail with one initial record
    // -------------------------------------------------------------------------
    let created = admin
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Trail opened"),
            Some("event:trail_created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;
    println!("Trail created: {trail_id}\n");

    // -------------------------------------------------------------------------
    // Step 2: Create a record-admin role and issue a capability for it
    // -------------------------------------------------------------------------
    admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;

    let capability = admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?
        .output;

    println!(
        "Issued capability {} for role {}\n",
        capability.capability_id, capability.role
    );

    // -------------------------------------------------------------------------
    // Step 3: Append follow-up records
    // -------------------------------------------------------------------------
    // The client automatically finds the capability in `record_admin`'s wallet.
    let records = record_admin.trail(trail_id).records();

    let first_added = records
        .add(
            Data::text("Shipment received at warehouse A"),
            Some("event:received".to_string()),
            None,
        )
        .build_and_execute(&record_admin)
        .await?
        .output;

    let second_added = records
        .add(
            Data::text("Shipment dispatched to retailer"),
            Some("event:dispatched".to_string()),
            None,
        )
        .build_and_execute(&record_admin)
        .await?
        .output;

    println!(
        "Added records at sequence numbers {} and {}\n",
        first_added.sequence_number, second_added.sequence_number
    );

    // -------------------------------------------------------------------------
    // Step 4: Read records back by sequence number
    // -------------------------------------------------------------------------
    let initial = records.get(0).await?;
    let first = records.get(first_added.sequence_number).await?;
    let second = records.get(second_added.sequence_number).await?;

    println!("Initial record: {:?}", initial.data);
    println!("First added record: {:?}", first.data);
    println!("Second added record: {:?}\n", second.data);

    ensure!(matches!(initial.data, Data::Text(ref text) if text == "Trail opened"));
    ensure!(matches!(
        first.data,
        Data::Text(ref text) if text == "Shipment received at warehouse A"
    ));
    ensure!(matches!(
        second.data,
        Data::Text(ref text) if text == "Shipment dispatched to retailer"
    ));

    // -------------------------------------------------------------------------
    // Step 5: Inspect record count and page through the linked table
    // -------------------------------------------------------------------------
    let count = records.record_count().await?;
    println!("Current record count: {count}");
    ensure!(count == 3, "expected 3 records, got {count}");

    let first_page = records.list_page(None, 2).await?;
    println!(
        "First page contains {} records; has_next_page = {}",
        first_page.records.len(),
        first_page.has_next_page
    );

    let second_page = records.list_page(first_page.next_cursor, 2).await?;
    println!(
        "Second page contains {} records; has_next_page = {}",
        second_page.records.len(),
        second_page.has_next_page
    );

    ensure!(first_page.records.len() == 2, "expected first page size 2");
    ensure!(second_page.records.len() == 1, "expected second page size 1");

    println!("\nRecord flow completed successfully.");

    Ok(())
}
