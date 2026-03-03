// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, LockingConfig, LockingWindow, Permission,
};
use iota_interaction::types::base_types::IotaAddress;
use product_common::core_client::CoreClient;

use crate::client::get_funded_test_client;

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

    let on_chain = created.fetch_audit_trail(&client).await?;
    assert_eq!(on_chain.id.object_id(), &created.trail_id);
    assert_eq!(on_chain.creator, client.sender_address());
    assert_eq!(on_chain.sequence_number, 1);
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::None
        }
    );
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
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 300 },
        })
        .with_trail_metadata(immutable_metadata.clone())
        .with_updatable_metadata("updatable metadata")
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let on_chain = created.fetch_audit_trail(&client).await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 300 }
        }
    );
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
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::CountBased { count: 3 },
        })
        .with_trail_metadata_parts("Trail Count Lock", Some("count lock description".to_string()))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let on_chain = created.fetch_audit_trail(&client).await?;
    assert_eq!(
        on_chain.locking_config,
        LockingConfig {
            delete_record: LockingWindow::CountBased { count: 3 }
        }
    );
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

    match cap {
        Ok(cap_ref) => println!("Found admin capability with ID: {}", cap_ref.0),
        Err(e) => println!("Error finding admin capability for custom admin: {e}"),
    }

    Ok(())
}

#[tokio::test]
async fn get_returns_on_chain_trail() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let created = client
        .create_trail()
        .with_initial_record(Data::text("trail-get-e2e"), None)
        .with_trail_metadata_parts("Get Test", Some("description".into()))
        .with_updatable_metadata("initial updatable")
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail = client.trail(created.trail_id);
    let on_chain = trail.get().await?;

    assert_eq!(on_chain.id.object_id(), &created.trail_id);
    assert_eq!(on_chain.creator, created.creator);
    assert_eq!(on_chain.sequence_number, 1);
    assert_eq!(
        on_chain.immutable_metadata,
        Some(ImmutableMetadata::new(
            "Get Test".to_string(),
            Some("description".to_string())
        ))
    );
    assert_eq!(on_chain.updatable_metadata, Some("initial updatable".to_string()));

    Ok(())
}

#[tokio::test]
async fn get_trail_without_metadata() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let created = client
        .create_trail()
        .with_initial_record(Data::text("trail-no-meta-e2e"), None)
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let on_chain = client.trail(created.trail_id).get().await?;

    assert!(on_chain.immutable_metadata.is_none());
    assert!(on_chain.updatable_metadata.is_none());

    Ok(())
}

#[tokio::test]
async fn migrate_is_available_on_trail_handle() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("trail-migrate-e2e")).await?;

    let handle_migrate = client.trail(trail_id).migrate().build_and_execute(&client).await;

    assert!(
        handle_migrate.is_err(),
        "new trails are already on latest package version, migrate should fail"
    );

    Ok(())
}

#[tokio::test]
async fn update_metadata_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let trail_id = client.create_test_trail(Data::text("trail-update-meta-e2e")).await?;
    // Set initial updatable metadata via update_metadata
    client
        .create_role(trail_id, "MetadataAdmin", vec![Permission::UpdateMetadata])
        .await?;
    client
        .issue_cap(trail_id, "MetadataAdmin", CapabilityIssueOptions::default())
        .await?;

    let trail = client.trail(trail_id);

    trail
        .update_metadata(Some("before".to_string()))
        .build_and_execute(&client)
        .await?;

    let before = trail.get().await?;
    assert_eq!(before.updatable_metadata, Some("before".to_string()));

    // Update to a new value
    trail
        .update_metadata(Some("after".to_string()))
        .build_and_execute(&client)
        .await?;

    let after = trail.get().await?;
    assert_eq!(after.updatable_metadata, Some("after".to_string()));

    Ok(())
}

#[tokio::test]
async fn update_metadata_to_none_clears_value() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let trail_id = client.create_test_trail(Data::text("trail-clear-meta-e2e")).await?;
    client
        .create_role(trail_id, "MetadataAdmin", vec![Permission::UpdateMetadata])
        .await?;
    client
        .issue_cap(trail_id, "MetadataAdmin", CapabilityIssueOptions::default())
        .await?;

    let trail = client.trail(trail_id);

    trail
        .update_metadata(Some("to-be-cleared".to_string()))
        .build_and_execute(&client)
        .await?;

    trail.update_metadata(None).build_and_execute(&client).await?;

    let on_chain = trail.get().await?;
    assert_eq!(on_chain.updatable_metadata, None);

    Ok(())
}

#[tokio::test]
async fn update_metadata_multiple_times() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    let trail_id = client.create_test_trail(Data::text("trail-multi-meta-e2e")).await?;
    client
        .create_role(trail_id, "MetadataAdmin", vec![Permission::UpdateMetadata])
        .await?;
    client
        .issue_cap(trail_id, "MetadataAdmin", CapabilityIssueOptions::default())
        .await?;

    let trail = client.trail(trail_id);

    // Set, then overwrite, then clear
    trail
        .update_metadata(Some("first".to_string()))
        .build_and_execute(&client)
        .await?;

    trail
        .update_metadata(Some("second".to_string()))
        .build_and_execute(&client)
        .await?;

    trail.update_metadata(None).build_and_execute(&client).await?;

    let on_chain = trail.get().await?;
    assert_eq!(on_chain.updatable_metadata, None);

    Ok(())
}

#[tokio::test]
async fn update_metadata_does_not_affect_immutable_metadata() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let immutable = ImmutableMetadata::new("Immutable Name".to_string(), Some("frozen".to_string()));

    let created = client
        .create_trail()
        .with_initial_record(Data::text("trail-immutable-check-e2e"), None)
        .with_trail_metadata(immutable.clone())
        .with_updatable_metadata("mutable")
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail_id = created.trail_id;
    client
        .create_role(trail_id, "MetadataAdmin", vec![Permission::UpdateMetadata])
        .await?;
    client
        .issue_cap(trail_id, "MetadataAdmin", CapabilityIssueOptions::default())
        .await?;

    let trail = client.trail(trail_id);

    trail
        .update_metadata(Some("changed".to_string()))
        .build_and_execute(&client)
        .await?;

    let on_chain = trail.get().await?;
    assert_eq!(on_chain.immutable_metadata, Some(immutable));
    assert_eq!(on_chain.updatable_metadata, Some("changed".to_string()));

    Ok(())
}

#[tokio::test]
async fn delete_audit_trail_fails_when_records_exist() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail(Data::text("trail-delete-not-empty-e2e"))
        .await?;
    client
        .create_role(trail_id, "TrailDeleteOnly", vec![Permission::DeleteAuditTrail])
        .await?;
    client
        .issue_cap(trail_id, "TrailDeleteOnly", CapabilityIssueOptions::default())
        .await?;
    let trail = client.trail(trail_id);

    let delete_result = trail.delete_audit_trail().build_and_execute(&client).await;
    assert!(delete_result.is_err(), "deleting a non-empty trail must fail");

    let on_chain = trail.get().await?;
    assert_eq!(on_chain.id.object_id(), &trail_id);

    Ok(())
}

#[tokio::test]
async fn delete_records_batch_then_delete_audit_trail_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let created = client
        .create_trail()
        .with_initial_record(Data::text("trail-batch-delete-e2e"), None)
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 3600 },
        })
        .finish()
        .build_and_execute(&client)
        .await?
        .output;
    client
        .create_role(
            created.trail_id,
            "TrailDeleteMaintenance",
            vec![Permission::DeleteAllRecords, Permission::DeleteAuditTrail],
        )
        .await?;
    client
        .issue_cap(
            created.trail_id,
            "TrailDeleteMaintenance",
            CapabilityIssueOptions::default(),
        )
        .await?;

    let trail = client.trail(created.trail_id);

    let deleted = trail
        .records()
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(deleted, 1, "initial record should be deleted in batch");
    assert_eq!(trail.records().record_count().await?, 0);

    let deleted_trail = trail.delete_audit_trail().build_and_execute(&client).await?.output;
    assert_eq!(deleted_trail.trail_id, created.trail_id);
    assert!(deleted_trail.timestamp > 0);

    let fetch_deleted = trail.get().await;
    assert!(
        fetch_deleted.is_err(),
        "trail object should no longer be readable after delete"
    );

    Ok(())
}
