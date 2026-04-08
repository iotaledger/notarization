// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Customs Clearance Example
//!
//! This example models a customs-clearance process for a single shipment.
//!
//! ## How the trail is used
//!
//! - `immutable_metadata`: shipment and declaration identity
//! - `updatable_metadata`: the current customs-processing status
//! - record tags: `documents`, `export`, `import`, and `inspection`
//! - roles and capabilities: each operational role writes only the events it owns
//! - locking: writes are frozen once the shipment is fully cleared

use anyhow::{Result, ensure};
use audit_trail::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, LockingConfig, LockingWindow, PermissionSet,
    RoleTags, TimeLock,
};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Customs Clearance ===\n");

    let client = get_funded_audit_trail_client().await?;

    println!("Creating a customs-clearance trail...");

    let created = client
        .create_trail()
        .with_record_tags(["documents", "export", "import", "inspection"])
        .with_trail_metadata(ImmutableMetadata::new(
            "Shipment SHP-2026-CLEAR-001".to_string(),
            Some("Route: Hamburg, Germany -> Nairobi, Kenya | Declaration: DEC-2026-44017".to_string()),
        ))
        .with_updatable_metadata("Status: Documents Pending")
        .with_locking_config(LockingConfig {
            delete_record_window: LockingWindow::CountBased { count: 2 },
            delete_trail_lock: TimeLock::None,
            write_lock: TimeLock::None,
        })
        .with_initial_record(InitialRecord::new(
            Data::text("Customs clearance case opened for inbound shipment"),
            Some("event:case_opened".to_string()),
            Some("documents".to_string()),
        ))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail_id = created.trail_id;

    issue_tagged_record_role(&client, trail_id, "DocsOperator", "documents", client.sender_address()).await?;
    issue_tagged_record_role(&client, trail_id, "ExportBroker", "export", client.sender_address()).await?;
    issue_tagged_record_role(&client, trail_id, "ImportBroker", "import", client.sender_address()).await?;

    client
        .trail(trail_id)
        .access()
        .for_role("Supervisor")
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    client
        .trail(trail_id)
        .access()
        .for_role("Supervisor")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&client)
        .await?;

    client
        .trail(trail_id)
        .access()
        .for_role("LockingAdmin")
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    client
        .trail(trail_id)
        .access()
        .for_role("LockingAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&client)
        .await?;

    let docs_uploaded = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Commercial invoice and packing list uploaded"),
            Some("event:documents_uploaded".to_string()),
            Some("documents".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    println!("Docs operator added record #{}.\n", docs_uploaded.sequence_number);

    client
        .trail(trail_id)
        .update_metadata(Some("Status: Awaiting Export Clearance".to_string()))
        .build_and_execute(&client)
        .await?;

    let export_filed = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Export declaration filed with German customs"),
            Some("event:export_declaration_filed".to_string()),
            Some("export".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    let export_cleared = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Export clearance granted by Hamburg customs office"),
            Some("event:export_cleared".to_string()),
            Some("export".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Export broker added records #{} and #{}.\n",
        export_filed.sequence_number, export_cleared.sequence_number
    );

    client
        .trail(trail_id)
        .update_metadata(Some("Status: Awaiting Import Clearance".to_string()))
        .build_and_execute(&client)
        .await?;

    let denied_inspection = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import broker attempted to record an inspection result"),
            Some("event:invalid_inspection_write".to_string()),
            Some("inspection".to_string()),
        )
        .build_and_execute(&client)
        .await;

    ensure!(
        denied_inspection.is_err(),
        "inspection-tagged writes should fail before an inspection-scoped capability exists"
    );
    println!("Inspection write was correctly denied before the inspector role existed.\n");

    issue_tagged_record_role(&client, trail_id, "Inspector", "inspection", client.sender_address()).await?;

    let inspection_done = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Customs inspection completed with no discrepancies"),
            Some("event:inspection_completed".to_string()),
            Some("inspection".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    println!("Inspector added record #{}.\n", inspection_done.sequence_number);

    let duty_assessed = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import duty assessed and paid"),
            Some("event:duty_assessed".to_string()),
            Some("import".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    let import_cleared = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import clearance granted by Nairobi customs"),
            Some("event:import_cleared".to_string()),
            Some("import".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Import broker added records #{} and #{}.\n",
        duty_assessed.sequence_number, import_cleared.sequence_number
    );

    client
        .trail(trail_id)
        .update_metadata(Some("Status: Cleared".to_string()))
        .build_and_execute(&client)
        .await?;

    client
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::Infinite)
        .build_and_execute(&client)
        .await?;

    let after_lock = client.trail(trail_id).get().await?;
    println!(
        "Write lock after clearance: {:?}\n",
        after_lock.locking_config.write_lock
    );

    let late_note = client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Late customs note after the case was closed"),
            Some("event:late_note".to_string()),
            Some("documents".to_string()),
        )
        .build_and_execute(&client)
        .await;

    ensure!(
        late_note.is_err(),
        "cleared customs trail should reject late writes after the final lock"
    );

    let trail = client.trail(trail_id);
    let first_page = trail.records().list_page(None, 20).await?;

    println!("Recorded customs events:");
    for (sequence_number, record) in &first_page.records {
        println!(
            "  #{} | {:?} | tag={:?} | {:?}",
            sequence_number, record.data, record.tag, record.metadata
        );
    }

    ensure!(first_page.records.len() == 6, "expected 6 customs records");
    ensure!(
        trail.get().await?.updatable_metadata.as_deref() == Some("Status: Cleared"),
        "customs case should finish in cleared state"
    );

    println!("\nCustoms clearance completed successfully.");

    Ok(())
}

async fn issue_tagged_record_role(
    client: &audit_trail::AuditTrailClient<product_common::test_utils::InMemSigner>,
    trail_id: iota_interaction::types::base_types::ObjectID,
    role_name: &str,
    tag: &str,
    issued_to: iota_interaction::types::base_types::IotaAddress,
) -> Result<()> {
    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .create(PermissionSet::record_admin_permissions(), Some(RoleTags::new([tag])))
        .build_and_execute(client)
        .await?;

    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(issued_to),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(client)
        .await?;

    Ok(())
}
