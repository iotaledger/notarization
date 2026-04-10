// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail, defines the FinanceWriter role restricted to the `finance` tag, and issues a
//!   capability bound to `finance_writer`'s address.
//! - **FinanceWriter**: Holds the address-bound capability. Can add `finance`-tagged records but is blocked from
//!   writing `legal`-tagged records.

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

    let admin = get_funded_audit_trail_client().await?;
    let finance_writer = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_record_tags(["finance", "legal"])
        .with_initial_record(InitialRecord::new(
            Data::text("Trail created"),
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
        .for_role("FinanceWriter")
        .create(
            audit_trail::core::types::PermissionSet {
                permissions: [Permission::AddRecord].into_iter().collect(),
            },
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&admin)
        .await?;

    let issued = admin
        .trail(trail_id)
        .access()
        .for_role("FinanceWriter")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(finance_writer.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?
        .output;

    println!(
        "Issued FinanceWriter capability {} to {}\n",
        issued.capability_id,
        finance_writer.sender_address()
    );

    // The client automatically scans `finance_writer`'s wallet for a capability object that
    // targets this trail and carries the required permission. No explicit capability ID is
    // needed — the lookup happens in the background on every operation.
    let finance_records = finance_writer.trail(trail_id).records();

    let added = finance_records
        .add(
            Data::text("Invoice approved"),
            Some("department:finance".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&finance_writer)
        .await?
        .output;

    println!(
        "Added tagged record at sequence number {} with tag \"finance\".\n",
        added.sequence_number
    );

    let wrong_tag = finance_records
        .add(
            Data::text("Legal review completed"),
            Some("department:legal".to_string()),
            Some("legal".to_string()),
        )
        .build_and_execute(&finance_writer)
        .await;

    ensure!(
        wrong_tag.is_err(),
        "a finance-scoped role must not add a legal-tagged record"
    );

    let finance_record = finance_records.get(added.sequence_number).await?;
    println!("Stored tagged record: {:?}", finance_record);

    ensure!(finance_record.tag.as_deref() == Some("finance"));

    Ok(())
}
