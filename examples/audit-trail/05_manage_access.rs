// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates and updates roles, issues capabilities, revokes and destroys them, and finally deletes the role
//!   once it is no longer needed.
//! - **OperationsUser**: The subject of all capability issuance. Capabilities are bound to this address to demonstrate
//!   that revocation immediately blocks their access.

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

    // `admin` manages roles and capability lifecycle.
    // `operations_user` represents the actor who receives (and later loses) access.
    let admin = get_funded_audit_trail_client().await?;
    let operations_user = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_initial_record(audit_trail::core::types::InitialRecord::new(
            Data::text("Trail created"),
            None,
            None,
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;

    let created_role = admin
        .trail(trail_id)
        .access()
        .for_role("Operations")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin)
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

    let updated_role = admin
        .trail(trail_id)
        .access()
        .for_role("Operations")
        .update_permissions(updated_permissions.clone(), None)
        .build_and_execute(&admin)
        .await?
        .output;
    println!("Updated role permissions: {:?}\n", updated_role.permissions.permissions);

    let constrained_capability = admin
        .trail(trail_id)
        .access()
        .for_role("Operations")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(operations_user.sender_address()),
            valid_from_ms: None,
            valid_until_ms: Some(4_102_444_800_000),
        })
        .build_and_execute(&admin)
        .await?
        .output;

    println!(
        "Issued constrained capability:\n  id = {}\n  issued_to = {:?}\n  valid_until = {:?}\n",
        constrained_capability.capability_id, constrained_capability.issued_to, constrained_capability.valid_until
    );

    let on_chain = admin.trail(trail_id).get().await?;
    let role_definition = on_chain.roles.roles.get("Operations").expect("role must exist");
    ensure!(role_definition.permissions == updated_permissions.permissions);

    admin
        .trail(trail_id)
        .access()
        .revoke_capability(constrained_capability.capability_id, constrained_capability.valid_until)
        .build_and_execute(&admin)
        .await?;
    println!("Revoked capability {}\n", constrained_capability.capability_id);

    // destroy_capability consumes the capability object, so the signer must own it.
    // The capability is issued to admin so admin can destroy it directly.
    let disposable_capability = admin
        .trail(trail_id)
        .access()
        .for_role("Operations")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?
        .output;

    admin
        .trail(trail_id)
        .access()
        .destroy_capability(disposable_capability.capability_id)
        .build_and_execute(&admin)
        .await?;
    println!("Destroyed capability {}\n", disposable_capability.capability_id);

    admin
        .trail(trail_id)
        .access()
        .cleanup_revoked_capabilities()
        .build_and_execute(&admin)
        .await?;
    println!("Cleaned up revoked capability registry entries.\n");

    admin
        .trail(trail_id)
        .access()
        .for_role("Operations")
        .delete()
        .build_and_execute(&admin)
        .await?;

    let after_delete = admin.trail(trail_id).get().await?;
    ensure!(
        !after_delete.roles.roles.contains_key("Operations"),
        "role should be removed from the trail"
    );

    println!("Removed the custom role after its capability lifecycle completed.");

    Ok(())
}
