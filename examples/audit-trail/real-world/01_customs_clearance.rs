// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Customs Clearance Example
//!
//! This example models a customs-clearance process for a single shipment.
//!
//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up all roles and capabilities.
//! - **Docs operator client**: Handles document submission (invoices, packing lists). Writes only `documents`-tagged
//!   records.
//! - **Export broker client**: Files export declarations and records clearance decisions at the origin. Writes only
//!   `export`-tagged records.
//! - **Import broker client**: Handles duty assessment and import clearance at the destination. Writes only
//!   `import`-tagged records.
//! - **Inspector client**: Records the outcome of a customs physical inspection. Writes only `inspection`-tagged
//!   records; the role is created mid-process when an inspection is triggered.
//! - **Supervisor client**: Updates the mutable trail metadata (processing status). No record-write permissions.
//! - **Locking admin client**: Freezes the trail once the shipment is fully cleared.
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
    TimeLock,
};
use examples::{get_funded_audit_trail_client, issue_tagged_record_role};
use product_common::core_client::CoreClient;
use sha2::{Digest, Sha256};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Customs Clearance ===\n");

    let admin_client = get_funded_audit_trail_client().await?;
    let docs_operator_client = get_funded_audit_trail_client().await?;
    let export_broker_client = get_funded_audit_trail_client().await?;
    let import_broker_client = get_funded_audit_trail_client().await?;
    let supervisor_client = get_funded_audit_trail_client().await?;
    let locking_admin_client = get_funded_audit_trail_client().await?;
    let inspector_client = get_funded_audit_trail_client().await?;

    // === Create the customs-clearance trail ===

    println!("Creating a customs-clearance trail...");

    let created_trail = admin_client
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
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;

    // === Set up roles and capabilities for each actor ===

    // The Admin capability delegates one tag-scoped writer role per operational actor.
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "DocsOperator",
        "documents",
        docs_operator_client.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "ExportBroker",
        "export",
        export_broker_client.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "ImportBroker",
        "import",
        import_broker_client.sender_address(),
    )
    .await?;

    let supervisor_role = "Supervisor";
    admin_client
        .trail(trail_id)
        .access()
        .for_role(supervisor_role)
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(supervisor_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(supervisor_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    let locking_admin_role = "LockingAdmin";
    admin_client
        .trail(trail_id)
        .access()
        .for_role(locking_admin_role)
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(locking_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(locking_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    // === Document submission ===

    // Documents are stored off-chain in an access-controlled environment (e.g. a TWIN node).
    // Only the SHA-256 fingerprint is committed on-chain for tamper-evidence.
    let invoice_hash = Sha256::digest(b"invoice-SHP-2026-CLEAR-001-v1.pdf");
    let docs_uploaded = docs_operator_client
        .trail(trail_id)
        .records()
        .add(
            Data::bytes(invoice_hash.to_vec()),
            Some("event:documents_uploaded".to_string()),
            Some("documents".to_string()),
        )
        .build_and_execute(&docs_operator_client)
        .await?
        .output;

    println!("Docs operator added record #{}.\n", docs_uploaded.sequence_number);

    supervisor_client
        .trail(trail_id)
        .update_metadata(Some("Status: Awaiting Export Clearance".to_string()))
        .build_and_execute(&supervisor_client)
        .await?;

    // === Export clearance ===

    let export_filed = export_broker_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Export declaration filed with German customs"),
            Some("event:export_declaration_filed".to_string()),
            Some("export".to_string()),
        )
        .build_and_execute(&export_broker_client)
        .await?
        .output;

    let export_cleared = export_broker_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Export clearance granted by Hamburg customs office"),
            Some("event:export_cleared".to_string()),
            Some("export".to_string()),
        )
        .build_and_execute(&export_broker_client)
        .await?
        .output;

    println!(
        "Export broker added records #{} and #{}.\n",
        export_filed.sequence_number, export_cleared.sequence_number
    );

    supervisor_client
        .trail(trail_id)
        .update_metadata(Some("Status: Awaiting Import Clearance".to_string()))
        .build_and_execute(&supervisor_client)
        .await?;

    // === Inspection gate ===

    // The import broker does not hold an inspection-scoped capability at this point.
    // The write attempt must fail to prove that tag-based access control is enforced.
    let denied_inspection_attempt = import_broker_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import broker attempted to record an inspection result"),
            Some("event:invalid_inspection_write".to_string()),
            Some("inspection".to_string()),
        )
        .build_and_execute(&import_broker_client)
        .await;

    ensure!(
        denied_inspection_attempt.is_err(),
        "inspection-tagged writes should fail before an inspection-scoped capability exists"
    );
    println!("Inspection write was correctly denied before the inspector role existed.\n");

    // A customs inspection is triggered; the inspector role is created and issued mid-process.
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "Inspector",
        "inspection",
        inspector_client.sender_address(),
    )
    .await?;

    let inspection_done = inspector_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Customs inspection completed with no discrepancies"),
            Some("event:inspection_completed".to_string()),
            Some("inspection".to_string()),
        )
        .build_and_execute(&inspector_client)
        .await?
        .output;

    println!("Inspector added record #{}.\n", inspection_done.sequence_number);

    // === Import clearance ===

    let duty_assessed = import_broker_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import duty assessed and paid"),
            Some("event:duty_assessed".to_string()),
            Some("import".to_string()),
        )
        .build_and_execute(&import_broker_client)
        .await?
        .output;

    let import_cleared = import_broker_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Import clearance granted by Nairobi customs"),
            Some("event:import_cleared".to_string()),
            Some("import".to_string()),
        )
        .build_and_execute(&import_broker_client)
        .await?
        .output;

    println!(
        "Import broker added records #{} and #{}.\n",
        duty_assessed.sequence_number, import_cleared.sequence_number
    );

    supervisor_client
        .trail(trail_id)
        .update_metadata(Some("Status: Cleared".to_string()))
        .build_and_execute(&supervisor_client)
        .await?;

    // === Final lock and verification ===

    locking_admin_client
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::Infinite)
        .build_and_execute(&locking_admin_client)
        .await?;

    let trail_after_lock = admin_client.trail(trail_id).get().await?;
    println!(
        "Write lock after clearance: {:?}\n",
        trail_after_lock.locking_config.write_lock
    );

    let late_note_attempt = docs_operator_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Late customs note after the case was closed"),
            Some("event:late_note".to_string()),
            Some("documents".to_string()),
        )
        .build_and_execute(&docs_operator_client)
        .await;

    ensure!(
        late_note_attempt.is_err(),
        "cleared customs trail should reject late writes after the final lock"
    );

    let admin_trail = admin_client.trail(trail_id);
    let customs_records_page = admin_trail.records().list_page(None, 20).await?;

    println!("Recorded customs events:");
    for (sequence_number, record) in &customs_records_page.records {
        println!(
            "  #{} | {:?} | tag={:?} | {:?}",
            sequence_number, record.data, record.tag, record.metadata
        );
    }

    ensure!(
        customs_records_page.records.len() == 7,
        "expected 7 customs records including the initial case-opened record"
    );
    ensure!(
        admin_trail.get().await?.updatable_metadata.as_deref() == Some("Status: Cleared"),
        "customs case should finish in cleared state"
    );

    println!("\nCustoms clearance completed successfully.");

    Ok(())
}
