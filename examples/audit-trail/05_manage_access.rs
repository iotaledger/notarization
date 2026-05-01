// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates and updates roles, issues capabilities, revokes and destroys them, and finally deletes
//!   the role once it is no longer needed.
//! - **Operations user client**: The subject of all capability issuance. Capabilities are bound to this address to
//!   demonstrate that revocation immediately blocks their access.

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

    // Use a separate operations client so capability ownership and revocation are visible.
    let admin_client = get_funded_audit_trail_client().await?;
    let operations_user_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_initial_record(audit_trail::core::types::InitialRecord::new(
            Data::text("Trail created"),
            None,
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let operations_role = "Operations";

    // The Admin capability authorizes the custom role definition.
    let created_operations_role = admin_client
        .trail(trail_id)
        .access()
        .for_role(operations_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?
        .output;
    println!("Created role: {}\n", created_operations_role.role);

    let updated_permissions = PermissionSet {
        permissions: HashSet::from([
            Permission::AddRecord,
            Permission::DeleteRecord,
            Permission::DeleteAllRecords,
        ]),
    };

    let updated_operations_role = admin_client
        .trail(trail_id)
        .access()
        .for_role(operations_role)
        .update_permissions(updated_permissions.clone(), None)
        .build_and_execute(&admin_client)
        .await?
        .output;
    println!(
        "Updated role permissions: {:?}\n",
        updated_operations_role.permissions.permissions
    );

    let operations_capability = admin_client
        .trail(trail_id)
        .access()
        .for_role(operations_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(operations_user_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: Some(4_102_444_800_000),
        })
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Issued constrained capability:\n  id = {}\n  issued_to = {:?}\n  valid_until = {:?}\n",
        operations_capability.capability_id, operations_capability.issued_to, operations_capability.valid_until
    );

    let on_chain_trail = admin_client.trail(trail_id).get().await?;
    let operations_role_definition = on_chain_trail
        .roles
        .roles
        .get(operations_role)
        .expect("role must exist");
    ensure!(operations_role_definition.permissions == updated_permissions.permissions);

    admin_client
        .trail(trail_id)
        .access()
        .revoke_capability(operations_capability.capability_id, operations_capability.valid_until)
        .build_and_execute(&admin_client)
        .await?;
    println!("Revoked capability {}\n", operations_capability.capability_id);

    // destroy_capability consumes the capability object, so the signer must own it.
    // This disposable capability is issued back to `admin_client` so it can be destroyed directly.
    let disposable_operations_capability = admin_client
        .trail(trail_id)
        .access()
        .for_role(operations_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?
        .output;

    admin_client
        .trail(trail_id)
        .access()
        .destroy_capability(disposable_operations_capability.capability_id)
        .build_and_execute(&admin_client)
        .await?;
    println!(
        "Destroyed capability {}\n",
        disposable_operations_capability.capability_id
    );

    admin_client
        .trail(trail_id)
        .access()
        .cleanup_revoked_capabilities()
        .build_and_execute(&admin_client)
        .await?;
    println!("Cleaned up revoked capability registry entries.\n");

    admin_client
        .trail(trail_id)
        .access()
        .for_role(operations_role)
        .delete()
        .build_and_execute(&admin_client)
        .await?;

    let trail_after_role_delete = admin_client.trail(trail_id).get().await?;
    ensure!(
        !trail_after_role_delete.roles.roles.contains_key(operations_role),
        "role should be removed from the trail"
    );

    println!("Removed the custom role after its capability lifecycle completed.");

    Ok(())
}
