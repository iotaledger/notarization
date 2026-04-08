// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, Permission, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create and update a custom role.
/// 2. Issue a constrained capability for that role.
/// 3. Revoke one capability and destroy another.
/// 4. Remove the role after its capability lifecycle is complete.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Manage Access ===\n");

    let client = get_funded_audit_trail_client().await?;
    let sender = client.sender_address();

    let created = client
        .create_trail()
        .with_initial_record(audit_trail::core::types::InitialRecord::new(
            Data::text("Trail created"),
            None,
            None,
        ))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail = client.trail(created.trail_id);
    let role = trail.access().for_role("Operations");

    let created_role = role
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&client)
        .await?
        .output;
    println!("Created role: {}\n", created_role.role);

    let updated_permissions = PermissionSet {
        permissions: HashSet::from([
            Permission::AddRecord,
            Permission::DeleteRecord,
            Permission::DeleteAllRecords,
        ]),
    };

    let updated_role = role
        .update_permissions(updated_permissions.clone(), None)
        .build_and_execute(&client)
        .await?
        .output;
    println!("Updated role permissions: {:?}\n", updated_role.permissions.permissions);

    let constrained_capability = role
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(sender),
            valid_from_ms: None,
            valid_until_ms: Some(4_102_444_800_000),
        })
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Issued constrained capability:\n  id = {}\n  issued_to = {:?}\n  valid_until = {:?}\n",
        constrained_capability.capability_id, constrained_capability.issued_to, constrained_capability.valid_until
    );

    let on_chain = trail.get().await?;
    let role_definition = on_chain.roles.roles.get("Operations").expect("role must exist");
    ensure!(role_definition.permissions == updated_permissions.permissions);

    trail
        .access()
        .revoke_capability(constrained_capability.capability_id, constrained_capability.valid_until)
        .build_and_execute(&client)
        .await?;
    println!("Revoked capability {}\n", constrained_capability.capability_id);

    let disposable_capability = role
        .issue_capability(Default::default())
        .build_and_execute(&client)
        .await?
        .output;

    trail
        .access()
        .destroy_capability(disposable_capability.capability_id)
        .build_and_execute(&client)
        .await?;
    println!("Destroyed capability {}\n", disposable_capability.capability_id);

    trail
        .access()
        .cleanup_revoked_capabilities()
        .build_and_execute(&client)
        .await?;
    println!("Cleaned up revoked capability registry entries.\n");

    role.delete().build_and_execute(&client).await?;

    let after_delete = trail.get().await?;
    ensure!(
        !after_delete.roles.roles.contains_key("Operations"),
        "role should be removed from the trail"
    );

    println!("Removed the custom role after its capability lifecycle completed.");

    Ok(())
}
