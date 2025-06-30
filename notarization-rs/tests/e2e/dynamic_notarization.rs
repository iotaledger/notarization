// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use iota_sdk::types::base_types::IotaAddress;
use notarization::core::types::{NotarizationMethod, State, TimeLock};
use product_common::core_client::CoreClientReadOnly;

use crate::client::get_funded_test_client;

#[tokio::test]
async fn create_simple_dynamic_notarization_works() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let onchain_notarization = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output;

    assert_eq!(
        onchain_notarization.immutable_metadata.description,
        Some("Test Notarization".to_string())
    );
    assert_eq!(onchain_notarization.immutable_metadata.locking, None);
    assert_eq!(onchain_notarization.updatable_metadata, None);
    assert_eq!(onchain_notarization.state_version_count, 0);
    assert_eq!(onchain_notarization.method, NotarizationMethod::Dynamic);
    Ok(())
}

#[tokio::test]
async fn test_dynamic_notarization_client_with_transfer_lock() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // unlock at tomorrow
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;

    assert!(is_transfer_locked);

    Ok(())
}

#[tokio::test]
async fn test_transfer_dynamic_notarization_client_with_transfer_lock_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // unlock at tomorrow
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;

    assert!(is_transfer_locked);
    let alice = IotaAddress::random_for_testing_only();

    let transfer_notarization = test_client
        .transfer_notarization(*notarization_id.object_id(), alice)
        .build_and_execute(&test_client)
        .await;

    assert!(transfer_notarization.is_err(), "transfer should fail");

    Ok(())
}

#[tokio::test]
async fn test_transfer_dynamic_notarization_client_with_no_transfer_lock_works() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;
    assert!(!is_transfer_locked);

    let alice = IotaAddress::random_for_testing_only();

    let transfer_notarization = test_client
        .transfer_notarization(*notarization_id.object_id(), alice)
        .build_and_execute(&test_client)
        .await;

    assert!(transfer_notarization.is_ok(), "transfer should succeed");

    Ok(())
}

#[tokio::test]
async fn test_update_state_dynamic_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("initial_state".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let new_state = State::from_string("updated_state".to_string(), Some("state_metadata".to_string()));

    let update_result = test_client
        .update_state(new_state.clone(), *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(update_result.is_ok(), "State update should succeed");

    let retrieved_state = test_client.state(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_state.data.as_text()?, "updated_state");
    assert_eq!(retrieved_state.metadata, Some("state_metadata".to_string()));

    let version_count = test_client.state_version_count(*notarization_id.object_id()).await?;
    assert_eq!(version_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_update_metadata_dynamic_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_updatable_metadata("initial_metadata".to_string())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let new_metadata = Some("updated_metadata".to_string());

    let update_result = test_client
        .update_metadata(new_metadata.clone(), *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(update_result.is_ok(), "Metadata update should succeed");

    let retrieved_metadata = test_client.updatable_metadata(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_metadata, new_metadata);

    let version_count = test_client.state_version_count(*notarization_id.object_id()).await?;
    assert_eq!(version_count, 0);

    Ok(())
}

#[tokio::test]
async fn test_destroy_dynamic_notarization_no_locks() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let destroy_result = test_client
        .destroy(*notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(
        destroy_result.is_ok(),
        "Destroy should succeed for unlocked notarization"
    );

    let res = test_client
        .get_object_ref_by_id(*notarization_id.object_id())
        .await
        .transpose();

    assert!(res.is_none(), "Notarization should be destroyed");

    Ok(())
}

#[tokio::test]
async fn test_destroy_dynamic_notarization_with_transfer_lock_fails() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let destroy_result = test_client
        .destroy(*notarization_id.object_id())
        .build_and_execute(&test_client)
        .await;

    assert!(destroy_result.is_err(), "Destroy should fail when transfer is locked");

    Ok(())
}

#[tokio::test]
async fn test_read_only_methods_dynamic_notarization() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let description = "Test Description".to_string();
    let updatable_metadata = "Test Metadata".to_string();

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string(
            "test_state".to_string(),
            Some("state_meta".to_string()),
        ))
        .with_immutable_description(description.clone())
        .with_updatable_metadata(updatable_metadata.clone())
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let retrieved_description = test_client.description(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_description, Some(description));

    let retrieved_metadata = test_client.updatable_metadata(*notarization_id.object_id()).await?;
    assert_eq!(retrieved_metadata, Some(updatable_metadata));

    let state = test_client.state(*notarization_id.object_id()).await?;
    assert_eq!(state.data.as_text()?, "test_state");
    assert_eq!(state.metadata, Some("state_meta".to_string()));

    let created_at = test_client.created_at_ts(*notarization_id.object_id()).await?;
    assert!(created_at > 0);

    let last_change = test_client.last_state_change_ts(*notarization_id.object_id()).await?;
    assert!(last_change > 0);

    let version_count = test_client.state_version_count(*notarization_id.object_id()).await?;
    assert_eq!(version_count, 0);

    let method = test_client.notarization_method(*notarization_id.object_id()).await?;
    assert_eq!(method, NotarizationMethod::Dynamic);

    Ok(())
}

#[tokio::test]
async fn test_lock_checking_methods() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let unlock_at = now_ts + 86400;

    let locked_notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .with_transfer_lock(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let unlocked_notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test_state".to_string(), None))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    assert!(
        test_client
            .is_transfer_locked(*locked_notarization_id.object_id())
            .await?
    );
    assert!(
        !test_client
            .is_transfer_locked(*unlocked_notarization_id.object_id())
            .await?
    );

    assert!(
        !test_client
            .is_update_locked(*locked_notarization_id.object_id())
            .await?
    );
    assert!(
        !test_client
            .is_update_locked(*unlocked_notarization_id.object_id())
            .await?
    );

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

    let lock_metadata_locked = test_client.lock_metadata(*locked_notarization_id.object_id()).await?;
    assert!(lock_metadata_locked.is_some());

    let lock_metadata_unlocked = test_client.lock_metadata(*unlocked_notarization_id.object_id()).await?;
    assert!(lock_metadata_unlocked.is_none());

    Ok(())
}

#[tokio::test]
async fn test_multiple_state_updates() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("state_v0".to_string(), None))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    for i in 1..=3 {
        let new_state = State::from_string(format!("state_v{i}"), Some(format!("metadata_{i}")));

        test_client
            .update_state(new_state, *notarization_id.object_id())
            .build_and_execute(&test_client)
            .await?;

        let version_count = test_client.state_version_count(*notarization_id.object_id()).await?;
        assert_eq!(version_count, i as u64);

        let state = test_client.state(*notarization_id.object_id()).await?;
        assert_eq!(state.data.as_text()?, format!("state_v{i}"));
        assert_eq!(state.metadata, Some(format!("metadata_{i}")));
    }

    Ok(())
}

#[tokio::test]
async fn test_bytes_state_operations() -> anyhow::Result<()> {
    let test_client = get_funded_test_client().await?;

    let initial_data = vec![1, 2, 3, 4, 5];
    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_bytes(initial_data.clone(), None))
        .finish()
        .build_and_execute(&test_client)
        .await?
        .output
        .id;

    let state = test_client.state(*notarization_id.object_id()).await?;
    assert_eq!(state.data.as_bytes()?, initial_data);

    let updated_data = vec![10, 20, 30];
    let new_state = State::from_bytes(updated_data.clone(), Some("bytes_metadata".to_string()));

    test_client
        .update_state(new_state, *notarization_id.object_id())
        .build_and_execute(&test_client)
        .await?;

    let updated_state = test_client.state(*notarization_id.object_id()).await?;
    assert_eq!(updated_state.data.as_bytes()?, updated_data);
    assert_eq!(updated_state.metadata, Some("bytes_metadata".to_string()));

    Ok(())
}
