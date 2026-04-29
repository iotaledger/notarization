// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and holds the built-in Admin capability minted on creation.
//! - **Record admin client**: Receives a RecordAdmin capability bound to their address so it can write records.

use anyhow::Result;
use audit_trail::core::types::{CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create an audit trail with an initial record and metadata.
/// 2. Inspect the built-in Admin role that is automatically granted to the creator.
/// 3. Use the Admin capability to define a `RecordAdmin` role.
/// 4. Issue a capability for the `RecordAdmin` role to a specific address.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Create Trail & Define Roles ===\n");

    // Use separate clients to show that admin rights and record-writing rights can belong to different addresses.
    let admin_client = get_funded_audit_trail_client().await?;
    let record_admin_client = get_funded_audit_trail_client().await?;

    println!("Admin client address:        {}", admin_client.sender_address());
    println!(
        "Record admin client address: {}\n",
        record_admin_client.sender_address()
    );

    // -------------------------------------------------------------------------
    // Step 1: Create an audit trail
    // -------------------------------------------------------------------------
    // The builder supports optional immutable metadata (name + description),
    // mutable updatable metadata, an initial record, record tag registry, and
    // locking configuration.
    //
    // On success, the transaction engine automatically mints an Admin capability
    // object and transfers it to the sender's address. This capability grants
    // full administrative control over the trail (role management, capability
    // issuance, tag management, etc.).
    let created_trail = admin_client
        .create_trail()
        .with_trail_metadata(ImmutableMetadata::new(
            "Product Shipment Audit Trail".to_string(),
            Some("Immutable audit log for product lifecycle events".to_string()),
        ))
        .with_updatable_metadata("Status: Active")
        .with_initial_record(InitialRecord::new(
            Data::text("Shipment #SHP-20260401-001 created at warehouse A"),
            Some("event:shipment_created;location:warehouse-a".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Trail created!\n  Trail ID:   {}\n  Creator:    {}\n  Timestamp:  {} ms\n",
        created_trail.trail_id, created_trail.creator, created_trail.timestamp
    );

    // Fetch the trail to inspect the role map that was initialized during creation.
    let on_chain_trail = admin_client.trail(created_trail.trail_id).get().await?;
    let admin_role_name = &on_chain_trail.roles.initial_admin_role_name;
    let admin_permissions = &on_chain_trail.roles.roles[admin_role_name].permissions;
    println!(
        "Built-in admin role: \"{admin_role_name}\" ({} permissions)\n",
        admin_permissions.len()
    );

    // -------------------------------------------------------------------------
    // Step 2: Define a RecordAdmin role
    // -------------------------------------------------------------------------
    // The Admin capability in `admin_client`'s wallet authorizes this role-management transaction.
    // This permission set is the standard bundle for adding, deleting, and correcting records.
    let record_admin_role = "RecordAdmin";
    let created_role = admin_client
        .trail(created_trail.trail_id)
        .access()
        .for_role(record_admin_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Role \"{}\" defined with permissions:\n  {:?}\n",
        created_role.role, created_role.permissions.permissions
    );

    // -------------------------------------------------------------------------
    // Step 3: Issue a capability for the RecordAdmin role
    // -------------------------------------------------------------------------
    // Issuing the capability delegates this role to `record_admin_client`; the Admin capability stays with
    // `admin_client`.
    let record_admin_capability = admin_client
        .trail(created_trail.trail_id)
        .access()
        .for_role(record_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Capability issued!\n  Capability ID: {}\n  Trail ID:      {}\n  Role:          {}\n  Issued to:     {}",
        record_admin_capability.capability_id,
        record_admin_capability.target_key,
        record_admin_capability.role,
        record_admin_capability
            .issued_to
            .map_or_else(|| "any holder (no address restriction)".to_string(), |a| a.to_string())
    );

    Ok(())
}
