// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin**: Creates the trail and sets up the LockingAdmin and RecordAdmin roles.
//! - **LockingAdmin**: Controls write and delete locks. Holds the LockingAdmin capability.
//! - **RecordAdmin**: Writes records. Used to demonstrate that the write lock is enforced per-sender, not just checked
//!   by the admin.

use anyhow::{Result, ensure};
use audit_trail::core::types::{CapabilityIssueOptions, Data, InitialRecord, LockingWindow, PermissionSet, TimeLock};
use examples::get_funded_audit_trail_client;
use product_common::core_client::CoreClient;

/// Demonstrates how to:
/// 1. Delegate locking updates through a `LockingAdmin` role.
/// 2. Freeze record creation with a write lock.
/// 3. Restore writes and add a new record.
/// 4. Update the delete-record window and delete-trail lock.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Configure Locking ===\n");

    // `admin` creates the trail and manages roles.
    // `locking_admin` controls write and delete locks.
    // `record_admin` writes records.
    let admin = get_funded_audit_trail_client().await?;
    let locking_admin = get_funded_audit_trail_client().await?;
    let record_admin = get_funded_audit_trail_client().await?;

    let created = admin
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Trail opened"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin)
        .await?
        .output;

    let trail_id = created.trail_id;

    admin
        .trail(trail_id)
        .access()
        .for_role("LockingAdmin")
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("LockingAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(locking_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin)
        .await?;
    admin
        .trail(trail_id)
        .access()
        .for_role("RecordAdmin")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin)
        .await?;

    locking_admin
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::Infinite)
        .build_and_execute(&locking_admin)
        .await?;

    let locked = admin.trail(trail_id).get().await?;
    println!("Write lock after update: {:?}\n", locked.locking_config.write_lock);
    ensure!(locked.locking_config.write_lock == TimeLock::Infinite);

    let blocked_add = record_admin
        .trail(trail_id)
        .records()
        .add(Data::text("This write should fail"), None, None)
        .build_and_execute(&record_admin)
        .await;
    ensure!(blocked_add.is_err(), "write lock should block adding records");

    locking_admin
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::None)
        .build_and_execute(&locking_admin)
        .await?;

    let added = record_admin
        .trail(trail_id)
        .records()
        .add(Data::text("Write lock lifted"), Some("event:resumed".to_string()), None)
        .build_and_execute(&record_admin)
        .await?
        .output;

    println!(
        "Added record {} after clearing the write lock.\n",
        added.sequence_number
    );

    locking_admin
        .trail(trail_id)
        .locking()
        .update_delete_record_window(LockingWindow::CountBased { count: 2 })
        .build_and_execute(&locking_admin)
        .await?;
    locking_admin
        .trail(trail_id)
        .locking()
        .update_delete_trail_lock(TimeLock::Infinite)
        .build_and_execute(&locking_admin)
        .await?;

    let final_state = admin.trail(trail_id).get().await?;
    println!(
        "Final locking config:\n  delete_record_window = {:?}\n  delete_trail_lock = {:?}\n  write_lock = {:?}",
        final_state.locking_config.delete_record_window,
        final_state.locking_config.delete_trail_lock,
        final_state.locking_config.write_lock
    );

    ensure!(final_state.locking_config.delete_record_window == LockingWindow::CountBased { count: 2 });
    ensure!(final_state.locking_config.delete_trail_lock == TimeLock::Infinite);
    ensure!(final_state.locking_config.write_lock == TimeLock::None);

    Ok(())
}
