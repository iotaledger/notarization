// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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

    let admin = get_funded_audit_trail_client().await?;
    let intended_writer = get_funded_audit_trail_client().await?;
    let wrong_writer = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_initial_record(InitialRecord::new(Data::text("Trail created"), None, None))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;

    admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;

    let issued = admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(intended_writer.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?
        .output;

    println!(
        "Issued capability {} to {}\n",
        issued.capability_id,
        intended_writer.sender_address()
    );

    let denied = wrong_writer
        .trail(trail_id)
        .records()
        .add(Data::text("Wrong writer"), None, None)
        .build_and_execute(&wrong_writer)
        .await;

    ensure!(
        denied.is_err(),
        "a capability bound to another address must not be usable"
    );

    let added = intended_writer
        .trail(trail_id)
        .records()
        .add(Data::text("Authorized writer"), None, None)
        .build_and_execute(&intended_writer)
        .await?
        .output;

    println!("Bound holder added record {} successfully.\n", added.sequence_number);

    admin
        .trail(trail_id)
        .access()
        .revoke_capability(issued.capability_id, issued.valid_until)
        .build_and_execute(&admin)
        .await?;

    let revoked_attempt = intended_writer
        .trail(trail_id)
        .records()
        .add(Data::text("Should fail after revoke"), None, None)
        .build_and_execute(&intended_writer)
        .await;

    ensure!(
        revoked_attempt.is_err(),
        "revoked capabilities must no longer authorize record writes"
    );

    println!(
        "Revoked capability {} and verified it can no longer be used.",
        issued.capability_id
    );

    Ok(())
}
