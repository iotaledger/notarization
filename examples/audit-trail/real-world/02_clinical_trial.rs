// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Clinical Trial Data-Integrity Example
//!
//! This example models a Phase III clinical trial where an immutable audit trail
//! guarantees data integrity, role-scoped access, and time-constrained oversight.
//!
//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up all roles and capabilities.
//! - **Enroller client**: Writes enrollment events. Restricted to the `enrollment` tag.
//! - **Safety officer client**: Records adverse events and safety observations. Restricted to `safety`.
//! - **Efficacy reviewer client**: Records treatment outcomes. Restricted to `efficacy`.
//! - **PK analyst client**: Records pharmacokinetic results. Restricted to the `pk` tag that is added mid-study when a
//!   PK sub-study is initiated.
//! - **Monitor client**: Updates the mutable study-phase metadata. Access is time-windowed to the active study period
//!   (90 days from now).
//! - **Data safety board client**: Controls write and delete locks. Freezes the dataset after review.
//! - **Regulator client**: Read-only verifier. In production this would use `AuditTrailClientReadOnly` (no signing
//!   key); here a funded client is used to keep the example self-contained.
//!
//! ## How the trail is used
//!
//! - `immutable_metadata`: protocol identity and study description
//! - `updatable_metadata`: current study phase (updated as the trial progresses)
//! - record tags: `enrollment`, `safety`, `efficacy`, `pk` (added mid-study)
//! - roles and capabilities: each role writes only its designated tag
//! - time-constrained capabilities: Monitor access is windowed to the study period
//! - locking: a deletion window protects recent records; a time-lock freezes the dataset after the Data Safety Board
//!   completes its review
//! - read-only verification: a regulator inspects the trail without write access

use anyhow::{Result, ensure};
use audit_trail::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, LockingConfig, LockingWindow, PermissionSet,
    TimeLock,
};
use examples::{get_funded_audit_trail_client, issue_tagged_record_role};
use product_common::core_client::CoreClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Clinical Trial Data Integrity ===\n");

    let admin_client = get_funded_audit_trail_client().await?;
    let enroller_client = get_funded_audit_trail_client().await?;
    let safety_officer_client = get_funded_audit_trail_client().await?;
    let efficacy_reviewer_client = get_funded_audit_trail_client().await?;
    let pk_analyst_client = get_funded_audit_trail_client().await?;
    let monitor_client = get_funded_audit_trail_client().await?;
    let data_safety_board_client = get_funded_audit_trail_client().await?;
    let regulator_client = get_funded_audit_trail_client().await?;

    // -----------------------------------------------------------------------
    // 1. Create the trial trail
    // -----------------------------------------------------------------------
    println!("Creating the clinical-trial audit trail...");

    let created_trail = admin_client
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
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    println!("Trail created with ID {trail_id}\n");

    // -----------------------------------------------------------------------
    // 2. Define roles with tag-scoped permissions
    // -----------------------------------------------------------------------
    println!("Defining study roles...");

    // The Admin capability delegates one tag-scoped writer role per study function.
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "Enroller",
        "enrollment",
        enroller_client.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "SafetyOfficer",
        "safety",
        safety_officer_client.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "EfficacyReviewer",
        "efficacy",
        efficacy_reviewer_client.sender_address(),
    )
    .await?;

    // Monitor can update metadata (study phase) but only during the study window.
    let monitor_role = "Monitor";
    admin_client
        .trail(trail_id)
        .access()
        .for_role(monitor_role)
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    // Monitor access is valid for 90 days from now.
    let study_end_ms = now_ms + 90 * 24 * 60 * 60 * 1000;

    admin_client
        .trail(trail_id)
        .access()
        .for_role(monitor_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(monitor_client.sender_address()),
            valid_from_ms: Some(now_ms),
            valid_until_ms: Some(study_end_ms),
        })
        .build_and_execute(&admin_client)
        .await?;

    println!("Monitor capability issued (valid for 90 days from now, ends at timestamp {study_end_ms})\n");

    // Data Safety Board can manage locking.
    let data_safety_board_role = "DataSafetyBoard";
    admin_client
        .trail(trail_id)
        .access()
        .for_role(data_safety_board_role)
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(data_safety_board_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(data_safety_board_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    // -----------------------------------------------------------------------
    // 3. Enrollment phase — add enrollment records
    // -----------------------------------------------------------------------
    println!("--- Enrollment Phase ---");

    let enrolled_record = enroller_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Patient P-101 enrolled at Site Hamburg"),
            Some("event:patient_enrolled".to_string()),
            Some("enrollment".to_string()),
        )
        .build_and_execute(&enroller_client)
        .await?
        .output;
    println!("Enroller added record #{}.\n", enrolled_record.sequence_number);

    // -----------------------------------------------------------------------
    // 4. Add safety and efficacy records
    // -----------------------------------------------------------------------
    println!("--- Study Data Collection ---");

    let safety_event_record = safety_officer_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Adverse event: mild headache reported by Patient P-101"),
            Some("event:adverse_event".to_string()),
            Some("safety".to_string()),
        )
        .build_and_execute(&safety_officer_client)
        .await?
        .output;

    let efficacy_outcome_record = efficacy_reviewer_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("Week 12: FEV1 improvement of 320 mL over baseline for P-101"),
            Some("event:efficacy_observed".to_string()),
            Some("efficacy".to_string()),
        )
        .build_and_execute(&efficacy_reviewer_client)
        .await?
        .output;

    println!(
        "SafetyOfficer added record #{}, EfficacyReviewer added record #{}.\n",
        safety_event_record.sequence_number, efficacy_outcome_record.sequence_number
    );

    // -----------------------------------------------------------------------
    // 5. Add a new tag mid-study (pharmacokinetics)
    // -----------------------------------------------------------------------
    println!("--- Mid-Study Amendment ---");

    // Admin adds the new tag and creates a role for the PK analyst.
    admin_client
        .trail(trail_id)
        .tags()
        .add("pk")
        .build_and_execute(&admin_client)
        .await?;
    println!("Added tag 'pk' (pharmacokinetics) to the trail.");

    issue_tagged_record_role(
        &admin_client,
        trail_id,
        "PkAnalyst",
        "pk",
        pk_analyst_client.sender_address(),
    )
    .await?;

    let pk_result_record = pk_analyst_client
        .trail(trail_id)
        .records()
        .add(
            Data::text("PK analysis: Cmax reached at 2.4 h, half-life 8.7 h"),
            Some("event:pk_result".to_string()),
            Some("pk".to_string()),
        )
        .build_and_execute(&pk_analyst_client)
        .await?
        .output;
    println!("PkAnalyst added record #{}.\n", pk_result_record.sequence_number);

    // -----------------------------------------------------------------------
    // 6. Deletion window protects recent records
    // -----------------------------------------------------------------------
    println!("--- Deletion Window Enforcement ---");

    let protected_delete_attempt = pk_analyst_client
        .trail(trail_id)
        .records()
        .delete(pk_result_record.sequence_number)
        .build_and_execute(&pk_analyst_client)
        .await;

    ensure!(
        protected_delete_attempt.is_err(),
        "recent records must be protected by the count-based deletion window"
    );
    println!(
        "Record #{} is within the deletion window (newest 3) and cannot be deleted.\n",
        pk_result_record.sequence_number
    );

    // -----------------------------------------------------------------------
    // 7. Monitor updates study phase metadata
    // -----------------------------------------------------------------------
    println!("--- Metadata Update ---");

    monitor_client
        .trail(trail_id)
        .update_metadata(Some("Phase: Data Review".to_string()))
        .build_and_execute(&monitor_client)
        .await?;

    let trail_after_phase_update = admin_client.trail(trail_id).get().await?;
    println!(
        "Study phase updated to: {:?}\n",
        trail_after_phase_update.updatable_metadata
    );

    // -----------------------------------------------------------------------
    // 8. Data Safety Board locks the study dataset
    // -----------------------------------------------------------------------
    println!("--- Data Safety Board Lock ---");

    // Lock writes until a specific future timestamp (e.g. 1 year from now),
    // after which the dataset becomes permanently locked.
    let lock_until_ms = now_ms + 365 * 24 * 60 * 60 * 1000; // 1 year from now

    data_safety_board_client
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::UnlockAtMs(lock_until_ms))
        .build_and_execute(&data_safety_board_client)
        .await?;

    let locked_trail = admin_client.trail(trail_id).get().await?;
    println!(
        "Write lock set to UnlockAtMs({}) — writes blocked until that timestamp.\n",
        lock_until_ms
    );
    println!("Current locking config: {:?}\n", locked_trail.locking_config);

    // Also lock the trail from deletion permanently.
    data_safety_board_client
        .trail(trail_id)
        .locking()
        .update_delete_trail_lock(TimeLock::Infinite)
        .build_and_execute(&data_safety_board_client)
        .await?;

    let final_locking_trail = admin_client.trail(trail_id).get().await?;
    println!(
        "Delete-trail lock set to {:?} — trail cannot be deleted.\n",
        final_locking_trail.locking_config.delete_trail_lock
    );

    // -----------------------------------------------------------------------
    // 9. Regulator read-only verification
    // -----------------------------------------------------------------------
    println!("--- Regulator Verification ---");

    // In production the regulator would use AuditTrailClientReadOnly (no signer).
    let regulator_trail = regulator_client.trail(trail_id);

    let on_chain_trail = regulator_trail.get().await?;
    println!("Protocol: {:?}", on_chain_trail.immutable_metadata);
    println!("Phase:     {:?}", on_chain_trail.updatable_metadata);
    println!("Roles:     {:?}", on_chain_trail.roles.roles.keys().collect::<Vec<_>>());
    println!(
        "Tags:      {:?}",
        on_chain_trail.tags.tag_map.keys().collect::<Vec<_>>()
    );

    let verified_records_page = regulator_trail.records().list_page(None, 20).await?;
    println!("\nVerified records ({} total):", verified_records_page.records.len());
    for (seq, record) in &verified_records_page.records {
        println!("  #{} | tag={:?} | {:?}", seq, record.tag, record.metadata);
    }

    // -----------------------------------------------------------------------
    // 10. Assertions
    // -----------------------------------------------------------------------
    ensure!(
        verified_records_page.records.len() == 5,
        "expected 5 records (initial + enrolled + safety + efficacy + pk)"
    );
    ensure!(
        on_chain_trail.tags.tag_map.contains_key("pk"),
        "the 'pk' tag must exist after mid-study amendment"
    );
    ensure!(
        on_chain_trail.locking_config.delete_record_window == LockingWindow::CountBased { count: 3 },
        "deletion window must remain count-based with count 3"
    );
    ensure!(
        on_chain_trail.locking_config.delete_trail_lock == TimeLock::Infinite,
        "delete-trail lock must be Infinite"
    );
    ensure!(
        matches!(on_chain_trail.locking_config.write_lock, TimeLock::UnlockAtMs(_)),
        "write lock must be UnlockAtMs"
    );
    ensure!(
        on_chain_trail.updatable_metadata.as_deref() == Some("Phase: Data Review"),
        "study phase must be 'Data Review'"
    );

    println!("\nClinical trial data-integrity verification completed successfully.");

    Ok(())
}
