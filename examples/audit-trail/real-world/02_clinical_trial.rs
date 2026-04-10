// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Clinical Trial Data-Integrity Example
//!
//! This example models a Phase III clinical trial where an immutable audit trail
//! guarantees data integrity, role-scoped access, and time-constrained oversight.
//!
//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up all roles and capabilities.
//! - **Enroller**: Writes enrollment events. Restricted to the `enrollment` tag.
//! - **SafetyOfficer**: Records adverse events and safety observations. Restricted to `safety`.
//! - **EfficacyReviewer**: Records treatment outcomes. Restricted to `efficacy`.
//! - **PkAnalyst**: Records pharmacokinetic results. Restricted to the `pk` tag that is added
//!   mid-study when a PK sub-study is initiated.
//! - **Monitor**: Updates the mutable study-phase metadata. Access is time-windowed to the
//!   active study period (90 days from now).
//! - **DataSafetyBoard**: Controls write and delete locks. Freezes the dataset after review.
//! - **Regulator**: Read-only verifier. In production this would use `AuditTrailClientReadOnly`
//!   (no signing key); here a funded client is used to keep the example self-contained.
//!
//! ## How the trail is used
//!
//! - `immutable_metadata`: protocol identity and study description
//! - `updatable_metadata`: current study phase (updated as the trial progresses)
//! - record tags: `enrollment`, `safety`, `efficacy`, `pk` (added mid-study)
//! - roles and capabilities: each role writes only its designated tag
//! - time-constrained capabilities: Monitor access is windowed to the study period
//! - locking: a deletion window protects recent records; a time-lock freezes the dataset after
//!   the Data Safety Board completes its review
//! - read-only verification: a regulator inspects the trail without write access

use anyhow::{Result, ensure};
use audit_trail::AuditTrailClient;
use audit_trail::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, LockingConfig, LockingWindow, PermissionSet,
    RoleTags, TimeLock,
};
use examples::get_funded_audit_trail_client;
use iota_sdk::types::base_types::{IotaAddress, ObjectID};
use product_common::core_client::CoreClient;
use product_common::test_utils::InMemSigner;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Clinical Trial Data Integrity ===\n");

    let admin = get_funded_audit_trail_client().await?;
    let enroller = get_funded_audit_trail_client().await?;
    let safety_officer = get_funded_audit_trail_client().await?;
    let efficacy_reviewer = get_funded_audit_trail_client().await?;
    let pk_analyst = get_funded_audit_trail_client().await?;
    let monitor = get_funded_audit_trail_client().await?;
    let data_safety_board = get_funded_audit_trail_client().await?;
    let regulator = get_funded_audit_trail_client().await?;

    // -----------------------------------------------------------------------
    // 1. Create the trial trail
    // -----------------------------------------------------------------------
    println!("Creating the clinical-trial audit trail...");

    let created = admin
        .create_trail()
        .with_record_tags(["enrollment", "safety", "efficacy"])
        .with_trail_metadata(ImmutableMetadata::new(
            "Protocol CTR-2026-03742".to_string(),
            Some("Phase III: Efficacy of Drug X vs Placebo in Moderate-to-Severe Asthma".to_string()),
        ))
        .with_updatable_metadata("Phase: Enrollment")
        .with_locking_config(LockingConfig {
            delete_record_window: LockingWindow::CountBased { count: 3 },
            delete_trail_lock: TimeLock::None,
            write_lock: TimeLock::None,
        })
        .with_initial_record(InitialRecord::new(
            Data::text("Clinical trial CTR-2026-03742 opened for enrollment"),
            Some("event:trial_opened".to_string()),
            Some("enrollment".to_string()),
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;
    println!("Trail created with ID {trail_id}\n");

    // -----------------------------------------------------------------------
    // 2. Define roles with tag-scoped permissions
    // -----------------------------------------------------------------------
    println!("Defining study roles...");

    issue_tagged_record_role(&admin, trail_id, "Enroller", "enrollment", enroller.sender_address()).await?;
    issue_tagged_record_role(&admin, trail_id, "SafetyOfficer", "safety", safety_officer.sender_address()).await?;
    issue_tagged_record_role(
        &admin,
        trail_id,
        "EfficacyReviewer",
        "efficacy",
        efficacy_reviewer.sender_address(),
    )
    .await?;

    // Monitor can update metadata (study phase) but only during the study window.
    admin
        .trail(trail_id)
        .access()
        .for_role("Monitor")
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    // Monitor access is valid for 90 days from now.
    let study_end_ms = now_ms + 90 * 24 * 60 * 60 * 1000;

    admin
        .trail(trail_id)
        .access()
        .for_role("Monitor")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(monitor.sender_address()),
            valid_from_ms: Some(now_ms),
            valid_until_ms: Some(study_end_ms),
        })
        .build_and_execute(&admin)
        .await?;

    println!("Monitor capability issued (valid for 90 days from now, ends at timestamp {study_end_ms})\n");

    // Data Safety Board can manage locking.
    admin
        .trail(trail_id)
        .access()
        .for_role("DataSafetyBoard")
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("DataSafetyBoard")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(data_safety_board.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    // -----------------------------------------------------------------------
    // 3. Enrollment phase — add enrollment records
    // -----------------------------------------------------------------------
    println!("--- Enrollment Phase ---");

    let enrolled = enroller
        .trail(trail_id)
        .records()
        .add(
            Data::text("Patient P-101 enrolled at Site Hamburg"),
            Some("event:patient_enrolled".to_string()),
            Some("enrollment".to_string()),
        )
        .build_and_execute(&enroller)
        .await?
        .output;
    println!("Enroller added record #{}.\n", enrolled.sequence_number);

    // -----------------------------------------------------------------------
    // 4. Add safety and efficacy records
    // -----------------------------------------------------------------------
    println!("--- Study Data Collection ---");

    let safety_event = safety_officer
        .trail(trail_id)
        .records()
        .add(
            Data::text("Adverse event: mild headache reported by Patient P-101"),
            Some("event:adverse_event".to_string()),
            Some("safety".to_string()),
        )
        .build_and_execute(&safety_officer)
        .await?
        .output;

    let efficacy_record = efficacy_reviewer
        .trail(trail_id)
        .records()
        .add(
            Data::text("Week 12: FEV1 improvement of 320 mL over baseline for P-101"),
            Some("event:efficacy_observed".to_string()),
            Some("efficacy".to_string()),
        )
        .build_and_execute(&efficacy_reviewer)
        .await?
        .output;

    println!(
        "SafetyOfficer added record #{}, EfficacyReviewer added record #{}.\n",
        safety_event.sequence_number, efficacy_record.sequence_number
    );

    // -----------------------------------------------------------------------
    // 5. Add a new tag mid-study (pharmacokinetics)
    // -----------------------------------------------------------------------
    println!("--- Mid-Study Amendment ---");

    // Admin adds the new tag and creates a role for the PK analyst.
    admin
        .trail(trail_id)
        .tags()
        .add("pk")
        .build_and_execute(&admin)
        .await?;
    println!("Added tag 'pk' (pharmacokinetics) to the trail.");

    issue_tagged_record_role(&admin, trail_id, "PkAnalyst", "pk", pk_analyst.sender_address()).await?;

    let pk_record = pk_analyst
        .trail(trail_id)
        .records()
        .add(
            Data::text("PK analysis: Cmax reached at 2.4 h, half-life 8.7 h"),
            Some("event:pk_result".to_string()),
            Some("pk".to_string()),
        )
        .build_and_execute(&pk_analyst)
        .await?
        .output;
    println!("PkAnalyst added record #{}.\n", pk_record.sequence_number);

    // -----------------------------------------------------------------------
    // 6. Deletion window protects recent records
    // -----------------------------------------------------------------------
    println!("--- Deletion Window Enforcement ---");

    let delete_attempt = pk_analyst
        .trail(trail_id)
        .records()
        .delete(pk_record.sequence_number)
        .build_and_execute(&pk_analyst)
        .await;

    ensure!(
        delete_attempt.is_err(),
        "recent records must be protected by the count-based deletion window"
    );
    println!(
        "Record #{} is within the deletion window (newest 3) and cannot be deleted.\n",
        pk_record.sequence_number
    );

    // -----------------------------------------------------------------------
    // 7. Monitor updates study phase metadata
    // -----------------------------------------------------------------------
    println!("--- Metadata Update ---");

    monitor
        .trail(trail_id)
        .update_metadata(Some("Phase: Data Review".to_string()))
        .build_and_execute(&monitor)
        .await?;

    let current_state = admin.trail(trail_id).get().await?;
    println!("Study phase updated to: {:?}\n", current_state.updatable_metadata);

    // -----------------------------------------------------------------------
    // 8. Data Safety Board locks the study dataset
    // -----------------------------------------------------------------------
    println!("--- Data Safety Board Lock ---");

    // Lock writes until a specific future timestamp (e.g. 1 year from now),
    // after which the dataset becomes permanently locked.
    let lock_until_ms = now_ms + 365 * 24 * 60 * 60 * 1000; // 1 year from now

    data_safety_board
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::UnlockAtMs(lock_until_ms))
        .build_and_execute(&data_safety_board)
        .await?;

    let locked_trail = admin.trail(trail_id).get().await?;
    println!(
        "Write lock set to UnlockAtMs({}) — writes blocked until that timestamp.\n",
        lock_until_ms
    );
    println!("Current locking config: {:?}\n", locked_trail.locking_config);

    // Also lock the trail from deletion permanently.
    data_safety_board
        .trail(trail_id)
        .locking()
        .update_delete_trail_lock(TimeLock::Infinite)
        .build_and_execute(&data_safety_board)
        .await?;

    let final_locking = admin.trail(trail_id).get().await?;
    println!(
        "Delete-trail lock set to {:?} — trail cannot be deleted.\n",
        final_locking.locking_config.delete_trail_lock
    );

    // -----------------------------------------------------------------------
    // 9. Regulator read-only verification
    // -----------------------------------------------------------------------
    println!("--- Regulator Verification ---");

    // In production the regulator would use AuditTrailClientReadOnly (no signer).
    let regulator_handle = regulator.trail(trail_id);

    let on_chain = regulator_handle.get().await?;
    println!("Protocol: {:?}", on_chain.immutable_metadata);
    println!("Phase:     {:?}", on_chain.updatable_metadata);
    println!("Roles:     {:?}", on_chain.roles.roles.keys().collect::<Vec<_>>());
    println!("Tags:      {:?}", on_chain.tags.tag_map.keys().collect::<Vec<_>>());

    let first_page = regulator_handle.records().list_page(None, 20).await?;
    println!("\nVerified records ({} total):", first_page.records.len());
    for (seq, record) in &first_page.records {
        println!("  #{} | tag={:?} | {:?}", seq, record.tag, record.metadata);
    }

    // -----------------------------------------------------------------------
    // 10. Assertions
    // -----------------------------------------------------------------------
    ensure!(
        first_page.records.len() == 5,
        "expected 5 records (initial + enrolled + safety + efficacy + pk)"
    );
    ensure!(
        on_chain.tags.tag_map.contains_key("pk"),
        "the 'pk' tag must exist after mid-study amendment"
    );
    ensure!(
        on_chain.locking_config.delete_record_window == LockingWindow::CountBased { count: 3 },
        "deletion window must remain count-based with count 3"
    );
    ensure!(
        on_chain.locking_config.delete_trail_lock == TimeLock::Infinite,
        "delete-trail lock must be Infinite"
    );
    ensure!(
        matches!(on_chain.locking_config.write_lock, TimeLock::UnlockAtMs(_)),
        "write lock must be UnlockAtMs"
    );
    ensure!(
        on_chain.updatable_metadata.as_deref() == Some("Phase: Data Review"),
        "study phase must be 'Data Review'"
    );

    println!("\nClinical trial data-integrity verification completed successfully.");

    Ok(())
}

async fn issue_tagged_record_role(
    client: &AuditTrailClient<InMemSigner>,
    trail_id: ObjectID,
    role_name: &str,
    tag: &str,
    issued_to: IotaAddress,
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
