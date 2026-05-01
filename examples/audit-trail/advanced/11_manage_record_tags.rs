// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and manages roles.
//! - **Tag admin client**: Holds the TagAdmin capability. Adds and removes entries from the trail's tag registry.
//! - **Finance writer client**: Holds a `finance`-scoped RecordAdmin capability. Writes a `finance`-tagged record that
//!   keeps the `finance` tag in use and therefore unremovable.

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

    // Use separate clients for registry management and tag-scoped record writing.
    let admin_client = get_funded_audit_trail_client().await?;
    let tag_admin_client = get_funded_audit_trail_client().await?;
    let finance_writer_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_record_tags(["finance"])
        .with_initial_record(InitialRecord::new(Data::text("Trail created"), None, None))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let tag_admin_role = "TagAdmin";
    let finance_writer_role = "FinanceWriter";

    admin_client
        .trail(trail_id)
        .access()
        .for_role(tag_admin_role)
        .create(PermissionSet::tag_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(tag_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(tag_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    tag_admin_client
        .trail(trail_id)
        .tags()
        .add("legal")
        .build_and_execute(&tag_admin_client)
        .await?;

    let trail_after_tag_add = admin_client.trail(trail_id).get().await?;
    println!(
        "Registry after adding \"legal\": {:?}\n",
        trail_after_tag_add.tags.tag_map
    );
    ensure!(trail_after_tag_add.tags.contains_key("finance"));
    ensure!(trail_after_tag_add.tags.contains_key("legal"));

    // FinanceWriter is scoped to the `finance` tag, which keeps that tag in use.
    admin_client
        .trail(trail_id)
        .access()
        .for_role(finance_writer_role)
        .create(
            PermissionSet::record_admin_permissions(),
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(finance_writer_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(finance_writer_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    finance_writer_client
        .trail(trail_id)
        .records()
        .add(Data::text("Tagged finance entry"), None, Some("finance".to_string()))
        .build_and_execute(&finance_writer_client)
        .await?;

    let remove_finance_attempt = tag_admin_client
        .trail(trail_id)
        .tags()
        .remove("finance")
        .build_and_execute(&tag_admin_client)
        .await;
    ensure!(
        remove_finance_attempt.is_err(),
        "a tag referenced by a role or record must not be removable"
    );

    tag_admin_client
        .trail(trail_id)
        .tags()
        .remove("legal")
        .build_and_execute(&tag_admin_client)
        .await?;

    let trail_after_tag_remove = admin_client.trail(trail_id).get().await?;
    println!(
        "Registry after removing \"legal\": {:?}\n",
        trail_after_tag_remove.tags.tag_map
    );

    ensure!(trail_after_tag_remove.tags.contains_key("finance"));
    ensure!(!trail_after_tag_remove.tags.contains_key("legal"));

    Ok(())
}
