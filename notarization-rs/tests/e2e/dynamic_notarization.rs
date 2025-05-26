// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use crate::client::get_funded_test_client;
use anyhow::anyhow;
use anyhow::Context;
use iota_interaction::IotaClientTrait;
use iota_sdk::rpc_types::{IotaData, IotaObjectDataOptions};
use iota_sdk::types::base_types::IotaAddress;
use notarization::core::notarization::OnChainNotarization;
use notarization::core::state::{Data, State};
use notarization::core::timelock::TimeLock;
use notarization::core::NotarizationMethod;

#[tokio::test]
async fn create_simple_dynamic_notarization_works() -> anyhow::Result<()> {
    let mut test_client = get_funded_test_client().await?;

    let onchain_notarization = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .finish()
        .build_and_execute(&mut test_client)
        .await?
        .output;

    println!("onchain_notarization: {:?}", onchain_notarization);

    assert_eq!(
        onchain_notarization.immutable_metadata.description,
        Some("Test Notarization".to_string())
    );
    assert_eq!(onchain_notarization.immutable_metadata.locking, None);
    assert_eq!(onchain_notarization.updateable_metadata, None);
    assert_eq!(onchain_notarization.state_version_count, 0);
    assert_eq!(onchain_notarization.method, NotarizationMethod::Dynamic);
    Ok(())
}

#[tokio::test]
async fn test_dynamic_notarization_client_with_transfer_lock() -> anyhow::Result<()> {
    let mut test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // unlock at tomorrow
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_at(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&mut test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;

    assert!(is_transfer_locked);

    Ok(())
}

#[tokio::test]
async fn test_transfer_dynamic_notarization_client_with_transfer_lock_fails() -> anyhow::Result<()> {
    let mut test_client = get_funded_test_client().await?;

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // unlock at tomorrow
    let unlock_at = now_ts + 86400;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_at(TimeLock::UnlockAt(unlock_at as u32))
        .finish()
        .build_and_execute(&mut test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;

    assert!(is_transfer_locked);
    let alice = IotaAddress::random_for_testing_only();

    let transfer_notarization = test_client
        .transfer_notarization(*notarization_id.object_id(), alice)
        .build_and_execute(&mut test_client)
        .await;

    assert!(transfer_notarization.is_err(), "transfer should fail");

    Ok(())
}

#[tokio::test]
async fn test_transfer_dynamic_notarization_client_with_no_transfer_lock_works() -> anyhow::Result<()> {
    let mut test_client = get_funded_test_client().await?;

    let notarization_id = test_client
        .create_dynamic_notarization()
        .with_state(State::from_string("test".to_string(), None))
        .with_immutable_description("Test Notarization".to_string())
        .with_transfer_at(TimeLock::None)
        .finish()
        .build_and_execute(&mut test_client)
        .await?
        .output
        .id;

    let is_transfer_locked = test_client.is_transfer_locked(*notarization_id.object_id()).await?;
    assert!(!is_transfer_locked);

    let alice = IotaAddress::random_for_testing_only();

    let transfer_notarization = test_client
        .transfer_notarization(*notarization_id.object_id(), alice)
        .build_and_execute(&mut test_client)
        .await;

    assert!(transfer_notarization.is_ok(), "transfer should succeed");

    Ok(())
}
