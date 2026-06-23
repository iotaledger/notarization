// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail, defines the RecordAdmin role, and issues a capability bound to
//!   `record_admin_client`'s address.
//! - **Record admin client**: Holds the capability. Appends a correction record, resolves the current record, and
//!   verifies that the original record cannot be corrected again.

use anyhow::{Result, ensure};
use audit_trails::core::types::{CapabilityIssueOptions, Data, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Append a correction record that supersedes an existing record.
/// 2. Read the original and correction records directly.
/// 3. Resolve the current record from the original sequence number.
/// 4. Show that an already replaced record cannot be corrected again.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail Advanced: Correct Records ===\n");

    let admin_client = get_funded_audit_trail_client().await?;
    let record_admin_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Invoice total: 100 USD"),
            Some("status:draft".to_string()),
            None,
        ))
        .finish()?
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let record_admin_role = "RecordAdmin";

    admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    let records = record_admin_client.trail(trail_id).records();

    let correction = records
        .correct(
            0,
            Data::text("Invoice total: 110 USD"),
            Some("status:corrected".to_string()),
            None,
        )
        .build_and_execute(&record_admin_client)
        .await?
        .output;

    println!(
        "Corrected record 0 by appending record {}.\n",
        correction.sequence_number
    );

    let original = records.get(0).await?;
    let correction_record = records.get(correction.sequence_number).await?;
    let current = records.resolve_current(0).await?;

    ensure!(
        original.correction.is_replaced_by == Some(correction.sequence_number),
        "the original record must point to the correction"
    );
    ensure!(
        correction_record.correction.replaces.contains(&0),
        "the correction must reference the original sequence number"
    );
    ensure!(
        current.sequence_number == correction.sequence_number,
        "resolve_current must return the appended correction"
    );
    ensure!(current.data == Data::text("Invoice total: 110 USD"));

    let second_correction_attempt = records
        .correct(
            0,
            Data::text("Invoice total: 120 USD"),
            Some("status:second-correction".to_string()),
            None,
        )
        .build_and_execute(&record_admin_client)
        .await;

    ensure!(
        second_correction_attempt.is_err(),
        "an already replaced record must not be corrected again"
    );

    println!("Original record: {:?}", original);
    println!("Correction record: {:?}", correction_record);
    println!("Current record resolved from #0: {:?}", current);

    Ok(())
}
