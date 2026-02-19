// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::types::{CapabilityIssueOptions, Data, LockingConfig, LockingWindow, Permission};
use iota_interaction::types::base_types::ObjectID;

use crate::client::{get_funded_test_client, TestClient};

async fn grant_role_capability(
    client: &TestClient,
    trail_id: ObjectID,
    role_name: &str,
    permissions: impl IntoIterator<Item = Permission>,
) -> anyhow::Result<()> {
    client.create_role(trail_id, role_name, permissions).await?;
    client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;
    Ok(())
}

#[tokio::test]
async fn update_locking_config_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("trail-update-locking-e2e")).await?;
    let trail = client.trail(trail_id);

    grant_role_capability(&client, trail_id, "LockingAdmin", [Permission::UpdateLockingConfig]).await?;

    trail
        .locking()
        .update(LockingConfig {
            delete_record: LockingWindow::CountBased { count: 2 },
        })
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::CountBased { count: 2 }
        }
    );

    Ok(())
}

#[tokio::test]
async fn update_locking_config_switches_count_to_time_based() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_trail()
        .with_initial_record(Data::text("trail-switch-count-to-time-e2e"), None)
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::CountBased { count: 3 },
        })
        .finish()
        .build_and_execute(&client)
        .await?
        .output
        .trail_id;
    let trail = client.trail(trail_id);

    grant_role_capability(&client, trail_id, "LockingAdmin", [Permission::UpdateLockingConfig]).await?;

    let before = trail.get().await?;
    assert_eq!(
        before.locking_config,
        LockingConfig {
            delete_record: LockingWindow::CountBased { count: 3 }
        }
    );

    trail
        .locking()
        .update(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 300 },
        })
        .build_and_execute(&client)
        .await?;

    let after = trail.get().await?;
    assert_eq!(
        after.locking_config,
        LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 300 }
        }
    );

    Ok(())
}

#[tokio::test]
async fn update_delete_record_window_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-update-delete-window-e2e"))
        .await?;
    let trail = client.trail(trail_id);

    grant_role_capability(
        &client,
        trail_id,
        "DeleteWindowAdmin",
        [Permission::UpdateLockingConfigForDeleteRecord],
    )
    .await?;

    trail
        .locking()
        .update_delete_record_window(LockingWindow::TimeBased { seconds: 120 })
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 120 }
        }
    );

    Ok(())
}

#[tokio::test]
async fn update_locking_config_requires_permission() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-locking-permission-e2e"))
        .await?;

    let result = client
        .trail(trail_id)
        .locking()
        .update(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 60 },
        })
        .build_and_execute(&client)
        .await;

    assert!(
        result.is_err(),
        "updating locking config without UpdateLockingConfig permission must fail"
    );

    Ok(())
}

#[tokio::test]
async fn is_record_locked_supports_count_window_and_missing_sequence() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_trail()
        .with_initial_record(Data::text("trail-locking-status-e2e"), None)
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::CountBased { count: 2 },
        })
        .finish()
        .build_and_execute(&client)
        .await?
        .output
        .trail_id;
    let trail = client.trail(trail_id);

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    trail
        .records()
        .add(Data::text("record-1"), None)
        .build_and_execute(&client)
        .await?;
    trail
        .records()
        .add(Data::text("record-2"), None)
        .build_and_execute(&client)
        .await?;

    assert!(
        !trail.locking().is_record_locked(0).await?,
        "oldest record should be unlocked with count window of 2 and total records of 3"
    );
    assert!(
        trail.locking().is_record_locked(2).await?,
        "latest record should be locked with count window of 2"
    );

    let missing = trail.locking().is_record_locked(999).await;
    assert!(missing.is_err(), "missing sequence should fail");

    Ok(())
}

#[tokio::test]
async fn delete_window_variants_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-locking-window-variants-e2e"))
        .await?;
    let trail = client.trail(trail_id);

    grant_role_capability(
        &client,
        trail_id,
        "DeleteWindowAdmin",
        [Permission::UpdateLockingConfigForDeleteRecord],
    )
    .await?;

    trail
        .locking()
        .update_delete_record_window(LockingWindow::TimeBased { seconds: 3600 })
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 3600 }
        }
    );

    trail
        .locking()
        .update_delete_record_window(LockingWindow::CountBased { count: 1 })
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::CountBased { count: 1 }
        }
    );

    trail
        .locking()
        .update_delete_record_window(LockingWindow::None)
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::None
        }
    );

    Ok(())
}

#[tokio::test]
async fn updated_time_lock_blocks_record_deletion() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-locking-delete-time-e2e"))
        .await?;
    let trail = client.trail(trail_id);

    grant_role_capability(
        &client,
        trail_id,
        "LockAndDeleteAdmin",
        [
            Permission::AddRecord,
            Permission::DeleteRecord,
            Permission::UpdateLockingConfig,
        ],
    )
    .await?;

    trail
        .records()
        .add("deletable-before-lock".into(), None)
        .build_and_execute(&client)
        .await?;

    trail
        .locking()
        .update(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 3600 },
        })
        .build_and_execute(&client)
        .await?;

    let delete_locked = trail.records().delete(1).build_and_execute(&client).await;
    assert!(
        delete_locked.is_err(),
        "deleting a record should fail after enabling a time-based delete lock"
    );
    assert_eq!(trail.records().record_count().await?, 2);

    Ok(())
}

#[tokio::test]
async fn updated_delete_window_can_block_and_then_allow_deletion() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-locking-delete-window-e2e"))
        .await?;
    let trail = client.trail(trail_id);

    grant_role_capability(
        &client,
        trail_id,
        "DeleteWindowAdmin",
        [Permission::DeleteRecord, Permission::UpdateLockingConfigForDeleteRecord],
    )
    .await?;

    trail
        .locking()
        .update_delete_record_window(LockingWindow::CountBased { count: 1 })
        .build_and_execute(&client)
        .await?;

    let delete_locked = trail.records().delete(0).build_and_execute(&client).await;
    assert!(
        delete_locked.is_err(),
        "count-based window should block deleting the latest record"
    );

    trail
        .locking()
        .update_delete_record_window(LockingWindow::None)
        .build_and_execute(&client)
        .await?;

    trail.records().delete(0).build_and_execute(&client).await?;
    assert_eq!(trail.records().record_count().await?, 0);

    Ok(())
}
