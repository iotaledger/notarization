// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, ensure};
use audit_trail::core::types::{Data, InitialRecord, LockingWindow, PermissionSet, TimeLock};
use examples::get_funded_audit_trail_client;

/// Demonstrates how to:
/// 1. Delegate locking updates through a `LockingAdmin` role.
/// 2. Freeze record creation with a write lock.
/// 3. Restore writes and add a new record.
/// 4. Update the delete-record window and delete-trail lock.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Audit Trail: Configure Locking ===\n");

    let client = get_funded_audit_trail_client().await?;

    let created = client
        .create_trail()
        .with_initial_record(InitialRecord::new(
            Data::text("Trail opened"),
            Some("event:created".to_string()),
            None,
        ))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail = client.trail(created.trail_id);

    trail
        .access()
        .for_role("LockingAdmin")
        .create(PermissionSet::locking_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("LockingAdmin")
        .issue_capability(Default::default())
        .build_and_execute(&client)
        .await?;

    trail
        .access()
        .for_role("RecordAdmin")
        .create(PermissionSet::record_admin_permissions(), None)
        .build_and_execute(&client)
        .await?;
    trail
        .access()
        .for_role("RecordAdmin")
        .issue_capability(Default::default())
        .build_and_execute(&client)
        .await?;

    trail
        .locking()
        .update_write_lock(TimeLock::Infinite)
        .build_and_execute(&client)
        .await?;

    let locked = trail.get().await?;
    println!("Write lock after update: {:?}\n", locked.locking_config.write_lock);
    ensure!(locked.locking_config.write_lock == TimeLock::Infinite);

    let blocked_add = trail
        .records()
        .add(Data::text("This write should fail"), None, None)
        .build_and_execute(&client)
        .await;
    ensure!(blocked_add.is_err(), "write lock should block adding records");

    trail
        .locking()
        .update_write_lock(TimeLock::None)
        .build_and_execute(&client)
        .await?;

    let added = trail
        .records()
        .add(Data::text("Write lock lifted"), Some("event:resumed".to_string()), None)
        .build_and_execute(&client)
        .await?
        .output;

    println!(
        "Added record {} after clearing the write lock.\n",
        added.sequence_number
    );

    trail
        .locking()
        .update_delete_record_window(LockingWindow::CountBased { count: 2 })
        .build_and_execute(&client)
        .await?;
    trail
        .locking()
        .update_delete_trail_lock(TimeLock::Infinite)
        .build_and_execute(&client)
        .await?;

    let final_state = trail.get().await?;
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
