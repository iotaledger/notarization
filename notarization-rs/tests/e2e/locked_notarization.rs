// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use iota_sdk::types::base_types::IotaAddress;
use notarization::core::types::{NotarizationMethod, State, TimeLock};
use product_common::core_client::CoreClientReadOnly;

use crate::client::get_funded_test_client;

#[tokio::test]
async fn create_simple_locked_notarization_works() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // unlock in 24 hours

    let onchain_notarization = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Locked Notarization".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output;

    assert_eq!(
        onchain_notarization.immutable_metadata.description,
        Some("Test Locked Notarization".to_string())
    );
    assert!(onchain_notarization.immutable_metadata.locking.is_some());
    assert_eq!(onchain_notarization.updatable_metadata, None);
    assert_eq!(onchain_notarization.state_version_count, 0);
    assert_eq!(onchain_notarization.method, NotarizationMethod::Locked);
    Ok(())
}

#[tokio::test]
async fn create_locked_notarization_with_updatable_metadata() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let onchain_notarization = test_client
        .create_locked_notarization()
        .with_state(State::from_string(
            "test_data".to_string(),
            Some("state_meta".to_string()),
        ))
        .with_immutable_description("Locked Document".to_string())
        .with_updatable_metadata("Initial metadata".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output;

    assert_eq!(
        onchain_notarization.updatable_metadata,
        Some("Initial metadata".to_string())
    );
    assert_eq!(onchain_notarization.method, NotarizationMethod::Locked);

    Ok(())
}

#[tokio::test]
async fn create_locked_notarization_with_none_delete_lock() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let onchain_notarization = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Never Locked Document".to_string())
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output;

    assert_eq!(onchain_notarization.method, NotarizationMethod::Locked);

    let is_destroy_allowed = test_client
        .is_destroy_allowed(*onchain_notarization.id.object_id())
        .await?;
    assert!(is_destroy_allowed, "destroy should be allowed with TimeLock::None");

    Ok(())
}

#[tokio::test]
async fn test_update_state_locked_notarization_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("initial_state".to_string(), None))
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let new_state = State::from_string("updated_state".to_string(), None);

    let update_result = test_client
        .update_state(new_state, *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(
        update_result.is_err(),
        "State update should fail on locked notarization"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_metadata_locked_notarization_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_updatable_metadata("initial_metadata".to_string())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let new_metadata = Some("updated_metadata".to_string());

    let update_result = test_client
        .update_metadata(new_metadata, *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(
        update_result.is_err(),
        "Metadata update should fail on locked notarization"
    );

    Ok(())
}

#[tokio::test]
async fn test_destroy_locked_notarization_before_unlock_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400; // unlock in 24 hours

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let destroy_result = test_client
        .destroy(*notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(
        destroy_result.is_err(),
        "Destroy should fail before delete lock expires"
    );

    Ok(())
}

#[tokio::test]
async fn test_destroy_locked_notarization_with_none_lock_succeeds() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let destroy_result = test_client
        .destroy(*notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(destroy_result.is_ok(), "Destroy should succeed with TimeLock::None");

    // check if the notarization is destroyed
    let res = test_client
        .get_object_ref_by_id(*notarization_id.object_id())
        .await
        .transpose();
    assert!(res.is_none(), "Notarization should be destroyed");

    Ok(())
}

#[tokio::test]
async fn test_read_only_methods_locked_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let description = "Locked Test Description".to_string();
    let updatable_metadata = "Locked Test Metadata".to_string();
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string(
            "locked_state".to_string(),
            Some("locked_state_meta".to_string()),
        ))
        .with_immutable_description(description.clone())
        .with_updatable_metadata(updatable_metadata.clone())
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let retrieved_description = test_client.description(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_description, Some(description));

    let retrieved_metadata = test_client.updatable_metadata(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_metadata, Some(updatable_metadata));

    let state = test_client.state(*notarization_id.object_id()).await?;
    assert_eq!(state.data.as_text()?, "locked_state");
    assert_eq!(state.metadata, Some("locked_state_meta".to_string()));

    let created_at = test_client.created_at_ts(*notarization_id.object_id()).await?;
    assert!(created_at > 0);

    let last_change = test_client.last_state_change_ts(*notarization_id.object_id()).await?;
    assert!(last_change > 0);

    let version_count = test_client.state_version_count(*notarization_id.object_id()).await?;
    assert_eq!(version_count, 0);

    let method = test_client.notarization_method(*notarization_id.object_id()).await?;
    assert_eq!(method, NotarizationMethod::Locked);

    Ok(())
}

#[tokio::test]
async fn test_lock_checking_methods_locked_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    // Locked notarization with time-based delete lock
    let locked_notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_delete_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    // Locked notarization with no delete lock
    let unlocked_notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    // For locked notarizations, update locks are always "until destroyed" (always locked)
    assert!(
        test_client
            .is_update_locked(*locked_notarization_id.object_id())
            .await?
    );
    assert!(
        test_client
            .is_update_locked(*unlocked_notarization_id.object_id())
            .await?
    );

    // Delete locks depend on the TimeLock setting
    assert!(
        !test_client
            .is_destroy_allowed(*locked_notarization_id.object_id())
            .await?
    );
    assert!(
        test_client
            .is_destroy_allowed(*unlocked_notarization_id.object_id())
            .await?
    );

    // For locked notarizations, transfer locks are always "until destroyed" (always locked)
    assert!(
        test_client
            .is_transfer_locked(*locked_notarization_id.object_id())
            .await?
    );
    assert!(
        test_client
            .is_transfer_locked(*unlocked_notarization_id.object_id())
            .await?
    );

    // Both should have lock metadata (locked notarizations always have lock metadata)
    let lock_metadata_locked = test_client.lock_metadata(*locked_notarization_id.object_id()).await?;
    assert!(lock_metadata_locked.is_some());

    let lock_metadata_unlocked = test_client.lock_metadata(*unlocked_notarization_id.object_id()).await?;
    assert!(lock_metadata_unlocked.is_some());

    Ok(())
}

#[tokio::test]
async fn test_bytes_state_operations_locked_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let initial_data = vec![91, 45, 30, 84, 76];
    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_bytes(initial_data.clone(), None))
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let state = test_client.state(*notarization_id.object_id()).await?;
    println!("state: {state:?}");
    assert_eq!(state.data.as_bytes()?, initial_data);

    // Attempting to update state should fail for locked notarization
    let updated_data = vec![10, 20, 30];
    let new_state = State::from_bytes(updated_data.clone(), Some("bytes_metadata".to_string()));

    let update_result = test_client
        .update_state(new_state, *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(
        update_result.is_err(),
        "State update should fail on locked notarization"
    );

    Ok(())
}

#[tokio::test]
async fn test_locked_notarization_transfer_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_delete_lock(TimeLock::None)
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let alice = IotaAddress::random_for_testing_only();

    // Transfer should fail because locked notarizations have transfer_lock = UntilDestroyed
    let transfer_result = test_client
        .transfer_notarization(*notarization_id.object_id(), alice)
        .build_and_execute(&test_client)
        .await;

    assert!(transfer_result.is_err(), "Transfer should fail for locked notarization");

    Ok(())
}

#[tokio::test]
async fn test_locked_notarization_different_delete_lock_times() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let short_unlock = now_ts + 3600; // 1 hour
    let long_unlock = now_ts + 86400; // 24 hours

    // Create notarization with short lock time
    let short_lock_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("short_lock".to_string(), None))
        .with_delete_lock(TimeLock::UnlockAt(short_unlock as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    // Create notarization with long lock time
    let long_lock_id = test_client
        .create_locked_notarization()
        .with_state(State::from_string("long_lock".to_string(), None))
        .with_delete_lock(TimeLock::UnlockAt(long_unlock as u32))
        .finish()?
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    // Both should be delete locked initially
    assert!(!test_client.is_destroy_allowed(*short_lock_id.object_id()).await?);
    assert!(!test_client.is_destroy_allowed(*long_lock_id.object_id()).await?);

    // Both should have lock metadata
    let short_lock_metadata = test_client.lock_metadata(*short_lock_id.object_id()).await?;
    let long_lock_metadata = test_client.lock_metadata(*long_lock_id.object_id()).await?;

    assert!(short_lock_metadata.is_some());
    assert!(long_lock_metadata.is_some());

    Ok(())
}
