// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and manages roles.
//! - **TagAdmin**: Holds the TagAdmin capability. Adds and removes entries from the trail's
//!   tag registry.
//! - **FinanceWriter**: Holds a `finance`-scoped RecordAdmin capability. Writes a
//!   `finance`-tagged record that keeps the `finance` tag in use and therefore
//!   unremovable.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, PermissionSet, RoleTags};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Delegate record-tag registry management to a `TagAdmin` role.
/// 2. Add and remove tags from the trail registry.
/// 3. Show that tags still in use by roles or records cannot be removed.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail Advanced: Manage Record Tags ===\n");

    // `admin` creates the trail and manages roles.
    // `tag_admin` adds and removes tags from the registry.
    // `finance_writer` holds a tag-scoped capability and writes finance records.
    let admin = get_funded_audit_trail_client().await?;
    let tag_admin = get_funded_audit_trail_client().await?;
    let finance_writer = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_record_tags(["finance"])
        .with_initial_record(InitialRecord::new(Data::text("Trail created"), None, None))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;

    admin
        .trail(trail_id)
        .access()
        .for_role("TagAdmin")
        .create(PermissionSet::tag_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("TagAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(tag_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    tag_admin.trail(trail_id).tags().add("legal").build_and_execute(&tag_admin).await?;

    let after_add = admin.trail(trail_id).get().await?;
    println!("Registry after adding \"legal\": {:?}\n", after_add.tags.tag_map);
    ensure!(after_add.tags.contains_key("finance"));
    ensure!(after_add.tags.contains_key("legal"));

    admin
        .trail(trail_id)
        .access()
        .for_role("FinanceWriter")
        .create(
            PermissionSet::record_admin_permissions(),
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("FinanceWriter")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(finance_writer.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    finance_writer
        .trail(trail_id)
        .records()
        .add(Data::text("Tagged finance entry"), None, Some("finance".to_string()))
        .build_and_execute(&finance_writer)
        .await?;

    let remove_finance = tag_admin.trail(trail_id).tags().remove("finance").build_and_execute(&tag_admin).await;
    ensure!(
        remove_finance.is_err(),
        "a tag referenced by a role or record must not be removable"
    );

    tag_admin.trail(trail_id).tags().remove("legal").build_and_execute(&tag_admin).await?;

    let after_remove = admin.trail(trail_id).get().await?;
    println!("Registry after removing \"legal\": {:?}\n", after_remove.tags.tag_map);

    ensure!(after_remove.tags.contains_key("finance"));
    ensure!(!after_remove.tags.contains_key("legal"));

    Ok(())
}
