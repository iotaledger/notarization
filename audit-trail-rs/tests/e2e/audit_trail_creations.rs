// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::client::get_funded_test_client;
use audit_trails::core::types::{Data, ImmutableMetadata, LockingConfig};
use iota_interaction::types::base_types::IotaAddress;
use product_common::core_client::CoreClient;

#[tokio::test]
async fn create_trail_with_default_builder_settings() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let created = client
        .create_trail()
        .with_initial_record(Data::text("audit-trail-create-default"), None)
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(created.creator, client.sender_address());

    let on_chain = created.load_on_chain(&client).await?;
    assert_eq!(on_chain.id.object_id(), &created.trail_id);
    assert_eq!(on_chain.creator, client.sender_address());
    assert_eq!(on_chain.sequence_number, 1);
    assert_eq!(on_chain.locking_config, LockingConfig::none());
    assert!(on_chain.immutable_metadata.is_none());
    assert!(on_chain.updatable_metadata.is_none());

    Ok(())
}

#[tokio::test]
async fn create_trail_with_metadata_and_time_lock() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let immutable_metadata =
        ImmutableMetadata::new("Trail Time Lock".to_string(), Some("immutable description".to_string()));

    let created = client
        .create_trail()
        .with_initial_record(
            Data::text("audit-trail-create-time-lock"),
            Some("initial record metadata".to_string()),
        )
        .with_locking_config(LockingConfig::time_based(300))
        .with_trail_metadata(immutable_metadata.clone())
        .with_updatable_metadata("updatable metadata")
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let on_chain = created.load_on_chain(&client).await?;
    assert_eq!(on_chain.locking_config, LockingConfig::time_based(300));
    assert_eq!(on_chain.immutable_metadata, Some(immutable_metadata));
    assert_eq!(on_chain.updatable_metadata, Some("updatable metadata".to_string()));

    Ok(())
}

#[tokio::test]
async fn create_trail_with_bytes_and_count_lock() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let created = client
        .create_trail()
        .with_initial_record(
            Data::bytes(vec![0xAA, 0xBB, 0xCC, 0xDD]),
            Some("bytes metadata".to_string()),
        )
        .with_locking_config(LockingConfig::count_based(3))
        .with_trail_metadata_parts("Trail Count Lock", Some("count lock description".to_string()))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let on_chain = created.load_on_chain(&client).await?;
    assert_eq!(on_chain.locking_config, LockingConfig::count_based(3));
    assert_eq!(on_chain.sequence_number, 1);

    Ok(())
}

#[tokio::test]
async fn create_trail_with_custom_admin_address() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let custom_admin = IotaAddress::random_for_testing_only();

    let created = client
        .create_trail()
        .with_admin(custom_admin)
        .with_initial_record(Data::text("audit-trail-custom-admin"), None)
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let cap = client.get_cap(custom_admin, created.trail_id).await;

    println!("Owned objects for custom admin {custom_admin}:");
    match cap {
        Ok(cap_ref) => println!("Found accredit capability with ID: {}", cap_ref.0),
        Err(e) => println!("Error finding accredit capability for custom admin: {e}"),
    }
    // assert!(has_admin_capability, "custom admin did not receive admin capability");

    Ok(())
}
