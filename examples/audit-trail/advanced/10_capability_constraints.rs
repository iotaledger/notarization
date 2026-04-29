// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail, defines the RecordAdmin role, and issues a capability bound specifically to
//!   `intended_writer_client`'s address. Also performs revocation.
//! - **Intended writer client**: The authorised holder. Writes a record successfully before revocation, then is blocked
//!   after the capability is revoked.
//! - **Wrong writer client**: An unauthorised actor who attempts to use the address-bound capability. All write
//!   attempts are rejected by the Move contract.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, PermissionSet};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Bind a capability to a specific wallet address.
/// 2. Show that a different wallet cannot use it.
/// 3. Revoke the capability and confirm the bound holder can no longer use it.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail Advanced: Capability Constraints ===\n");

    let admin_client = get_funded_audit_trail_client().await?;
    let intended_writer_client = get_funded_audit_trail_client().await?;
    let wrong_writer_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_initial_record(InitialRecord::new(Data::text("Trail created"), None, None))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let record_admin_role = "RecordAdmin";

    admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;

    // Address binding means only `intended_writer_client` may use the capability object.
    let intended_writer_capability = admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(intended_writer_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?
        .output;

    println!(
        "Issued capability {} to {}\n",
        intended_writer_capability.capability_id,
        intended_writer_client.sender_address()
    );

    let wrong_writer_attempt = wrong_writer_client
        .trail(trail_id)
        .records()
        .add(Data::text("Wrong writer"), None, None)
        .build_and_execute(&wrong_writer_client)
        .await;

    ensure!(
        wrong_writer_attempt.is_err(),
        "a capability bound to another address must not be usable"
    );

    let authorized_record = intended_writer_client
        .trail(trail_id)
        .records()
        .add(Data::text("Authorized writer"), None, None)
        .build_and_execute(&intended_writer_client)
        .await?
        .output;

    println!(
        "Bound holder added record {} successfully.\n",
        authorized_record.sequence_number
    );

    admin_client
        .trail(trail_id)
        .access()
        .revoke_capability(
            intended_writer_capability.capability_id,
            intended_writer_capability.valid_until,
        )
        .build_and_execute(&admin_client)
        .await?;

    let revoked_capability_attempt = intended_writer_client
        .trail(trail_id)
        .records()
        .add(Data::text("Should fail after revoke"), None, None)
        .build_and_execute(&intended_writer_client)
        .await;

    ensure!(
        revoked_capability_attempt.is_err(),
        "revoked capabilities must no longer authorize record writes"
    );

    println!(
        "Revoked capability {} and verified it can no longer be used.",
        intended_writer_capability.capability_id
    );

    Ok(())
}
