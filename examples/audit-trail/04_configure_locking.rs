// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! ## Actors
//!
//! - **Admin client**: Creates the trail and sets up the LockingAdmin and RecordAdmin roles.
//! - **Locking admin client**: Controls write and delete locks. Holds the LockingAdmin capability.
//! - **Record admin client**: Writes records. Used to demonstrate that the write lock is enforced per-sender, not just
//!   checked by the admin.

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

    // Use separate clients to show that locking and record-writing permissions can be delegated independently.
    let admin_client = get_funded_audit_trail_client().await?;
    let locking_admin_client = get_funded_audit_trail_client().await?;
    let record_admin_client = get_funded_audit_trail_client().await?;

    let created_trail = admin_client
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Trail opened"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&admin_client)
        .await?
        .output;

    let trail_id = created_trail.trail_id;
    let locking_admin_role = "LockingAdmin";
    let record_admin_role = "RecordAdmin";

    // The Admin capability authorizes defining roles and issuing the delegated capabilities.
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

    admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&admin_client)
        .await?;
    admin_client
        .trail(trail_id)
        .access()
        .for_role(record_admin_role)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(record_admin_client.sender_address()),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(&admin_client)
        .await?;

    locking_admin_client
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::Infinite)
        .build_and_execute(&locking_admin_client)
        .await?;

    let locked_trail = admin_client.trail(trail_id).get().await?;
    println!(
        "Write lock after update: {:?}\n",
        locked_trail.locking_config.write_lock
    );
    ensure!(locked_trail.locking_config.write_lock == TimeLock::Infinite);

    let blocked_add = record_admin_client
        .trail(trail_id)
        .records()
        .add(Data::text("This write should fail"), None, None)
        .build_and_execute(&record_admin_client)
        .await;
    ensure!(blocked_add.is_err(), "write lock should block adding records");

    locking_admin_client
        .trail(trail_id)
        .locking()
        .update_write_lock(TimeLock::None)
        .build_and_execute(&locking_admin_client)
        .await?;

    let added_record = record_admin_client
        .trail(trail_id)
        .records()
        .add(Data::text("Write lock lifted"), Some("event:resumed".to_string()), None)
        .build_and_execute(&record_admin_client)
        .await?
        .output;

    println!(
        "Added record {} after clearing the write lock.\n",
        added_record.sequence_number
    );

    locking_admin_client
        .trail(trail_id)
        .locking()
        .update_delete_record_window(LockingWindow::CountBased { count: 2 })
        .build_and_execute(&locking_admin_client)
        .await?;
    locking_admin_client
        .trail(trail_id)
        .locking()
        .update_delete_trail_lock(TimeLock::Infinite)
        .build_and_execute(&locking_admin_client)
        .await?;

    let final_trail = admin_client.trail(trail_id).get().await?;
    println!(
        "Final locking config:\n  delete_record_window = {:?}\n  delete_trail_lock = {:?}\n  write_lock = {:?}",
        final_trail.locking_config.delete_record_window,
        final_trail.locking_config.delete_trail_lock,
        final_trail.locking_config.write_lock
    );

    ensure!(final_trail.locking_config.delete_record_window == LockingWindow::CountBased { count: 2 });
    ensure!(final_trail.locking_config.delete_trail_lock == TimeLock::Infinite);
    ensure!(final_trail.locking_config.write_lock == TimeLock::None);

    Ok(())
}
