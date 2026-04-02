// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use audit_trail::core::types::{CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Create an audit trail with an initial record and metadata.
/// 2. Inspect the built-in Admin role that is automatically granted to the creator.
/// 3. Use the Admin capability to define a `RecordAdmin` role.
/// 4. Issue a capability for the `RecordAdmin` role.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Create Trail & Define Roles ===\n");

    // Create a funded client. The client's sender address becomes the initial Admin
    // of any trail it creates.
    let client = get_funded_audit_trail_client().await?;
    println!("Client address: {}", client.sender_address());

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
    let created = client
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
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Trail created!\n  Trail ID:   {}\n  Creator:    {}\n  Timestamp:  {} ms\n",
        created.trail_id, created.creator, created.timestamp
    );

    // Fetch the on-chain trail object to inspect the automatically created Admin role.
    let trail = client.trail(created.trail_id).get().await?;
    let admin_role_name = &trail.roles.initial_admin_role_name;
    let admin_permissions = &trail.roles.roles[admin_role_name].permissions;
    println!(
        "Built-in admin role: \"{admin_role_name}\" ({} permissions)\n",
        admin_permissions.len()
    );

    // -------------------------------------------------------------------------
    // Step 2: Define a RecordAdmin role
    // -------------------------------------------------------------------------
    // The Admin capability (held by the sender) allows creating new roles.
    // PermissionSet::record_admin_permissions() grants AddRecord, DeleteRecord,
    // and CorrectRecord permissions.
    let record_admin_role = "RecordAdmin";
    let role_created = client
        .trail(created.trail_id)
        .access()
        .for_role(record_admin_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Role \"{}\" defined with permissions:\n  {:?}\n",
        role_created.role, role_created.permissions.permissions
    );

    // -------------------------------------------------------------------------
    // Step 3: Issue a capability for the RecordAdmin role
    // -------------------------------------------------------------------------
    // A Capability object is minted on-chain and sent to the caller's address
    // (or a specified `issued_to` address via CapabilityIssueOptions).
    // The holder of this capability can add, delete, and correct records on the trail.
    let capability = client
        .trail(created.trail_id)
        .access()
        .for_role(record_admin_role)
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Capability issued!\n  Capability ID: {}\n  Trail ID:      {}\n  Role:          {}\n  Issued to:     {}",
        capability.capability_id,
        capability.target_key,
        capability.role,
        capability
            .issued_to
            .map_or_else(|| "any holder (no address restriction)".to_string(), |a| a.to_string())
    );

    Ok(())
}