// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, PermissionSet, RoleTags};
use examples::get_funded_audit_trail_client;

/// Demonstrates how to:
/// 1. Delegate record-tag registry management to a `TagAdmin` role.
/// 2. Add and remove tags from the trail registry.
/// 3. Show that tags still in use by roles or records cannot be removed.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail Advanced: Manage Record Tags ===\n");

    let client = get_funded_audit_trail_client().await?;

    let created = client
        .create_trail()
        .with_record_tags(["finance"])
        .with_initial_record(InitialRecord::new(Data::text("Trail created"), None, None))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail = client.trail(created.trail_id);

    trail
        .access()
        .for_role("TagAdmin")
        .create(PermissionSet::tag_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("TagAdmin")
        .issue_capability(Default::default())
        .build_and_execute(&client)
        .await?;

    trail.tags().add("legal").build_and_execute(&client).await?;

    let after_add = trail.get().await?;
    println!("Registry after adding \"legal\": {:?}\n", after_add.tags.tag_map);
    ensure!(after_add.tags.contains_key("finance"));
    ensure!(after_add.tags.contains_key("legal"));

    trail
        .access()
        .for_role("FinanceWriter")
        .create(
            PermissionSet::record_admin_permissions(),
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("FinanceWriter")
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?;

    trail
        .records()
        .add(Data::text("Tagged finance entry"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await?;

    let remove_finance = trail.tags().remove("finance").build_and_execute(&client).await;
    ensure!(
        remove_finance.is_err(),
        "a tag referenced by a role or record must not be removable"
    );

    trail.tags().remove("legal").build_and_execute(&client).await?;

    let after_remove = trail.get().await?;
    println!("Registry after removing \"legal\": {:?}\n", after_remove.tags.tag_map);

    ensure!(after_remove.tags.contains_key("finance"));
    ensure!(!after_remove.tags.contains_key("legal"));

    Ok(())
}
