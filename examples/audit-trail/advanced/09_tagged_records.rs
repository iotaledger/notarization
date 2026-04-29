// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail, defines the FinanceWriter role restricted to the `finance` tag, and issues a
//!   capability bound to `finance_writer_client`'s address.
//! - **Finance writer client**: Holds the address-bound capability. Can add `finance`-tagged records but is blocked
//!   from writing `legal`-tagged records.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, Permission, RoleTags};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create a trail with a predefined tag registry.
/// 2. Define a role that is restricted to one record tag.
/// 3. Issue a capability bound to a specific wallet address.
/// 4. Show that the holder can add only records matching the allowed tag.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail Advanced: Tagged Records ===\n");

    let admin_client = get_funded_audit_trail_client().await?;
    let finance_writer_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_record_tags(["finance", "legal"])
        .with_initial_record(InitialRecord::new(
            Data::text("Trail created"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let finance_writer_role = "FinanceWriter";

    // The role's tag scope limits writes even when the holder has AddRecord permission.
    admin_client
        .trail(trail_id)
        .access()
        .for_role(finance_writer_role)
        .create(
            audit_trail::core::types::PermissionSet {
                permissions: [Permission::AddRecord].into_iter().collect(),
            },
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&admin_client)
        .await?;

    let finance_writer_capability = admin_client
        .trail(trail_id)
        .access()
        .for_role(finance_writer_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(finance_writer_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Issued FinanceWriter capability {} to {}\n",
        finance_writer_capability.capability_id,
        finance_writer_client.sender_address()
    );

    // The client automatically scans `finance_writer_client`'s wallet for a capability object that
    // targets this trail and carries the required permission. No explicit capability ID is
    // needed — the lookup happens in the background on every operation.
    let finance_records = finance_writer_client.trail(trail_id).records();

    let finance_record_added = finance_records
        .add(
            Data::text("Invoice approved"),
            Some("department:finance".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&finance_writer_client)
        .await?
        .output;

    println!(
        "Added tagged record at sequence number {} with tag \"finance\".\n",
        finance_record_added.sequence_number
    );

    let wrong_tag_attempt = finance_records
        .add(
            Data::text("Legal review completed"),
            Some("department:legal".to_string()),
            Some("legal".to_string()),
        )
        .build_and_execute(&finance_writer_client)
        .await;

    ensure!(
        wrong_tag_attempt.is_err(),
        "a finance-scoped role must not add a legal-tagged record"
    );

    let finance_record = finance_records.get(finance_record_added.sequence_number).await?;
    println!("Stored tagged record: {:?}", finance_record);

    ensure!(finance_record.tag.as_deref() == Some("finance"));

    Ok(())
}
