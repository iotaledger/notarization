// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use audit_trail::core::types::{
    CapabilityIssueOptions, Data, InitialRecord, LockingConfig, LockingWindow, Permission, RoleTags, TimeLock,
};
use audit_trail::error::Error;
use iota_interaction::types::base_types::ObjectID;
use product_common::core_client::CoreClient;
use tokio::time::{Duration, sleep};

use crate::client::{TestClient, get_funded_test_client};

async fn grant_role_capability(
    client: &TestClient,
    trail_id: ObjectID,
    role_name: &str,
    permissions: impl IntoIterator<Item = Permission>,
) -> anyhow::Result<()> {
    client.create_role(trail_id, role_name, permissions, None).await?;
    client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;
    Ok(())
}

fn assert_text_data(data: Data, expected: &str) {
    match data {
        Data::Text(actual) => assert_eq!(actual, expected),
        other => panic!("expected text data, got {other:?}"),
    }
}

fn assert_bytes_data(data: Data, expected: &[u8]) {
    match data {
        Data::Bytes(actual) => assert_eq!(actual, expected),
        other => panic!("expected bytes data, got {other:?}"),
    }
}

fn config_with_window(delete_record_window: LockingWindow) -> LockingConfig {
    LockingConfig {
        delete_record_window,
        delete_trail_lock: TimeLock::None,
        write_lock: TimeLock::None,
    }
}

#[tokio::test]
async fn add_and_fetch_record_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("records-e2e")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    let added = records
        .add(Data::text("second record"), Some("second metadata".to_string()), None)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.trail_id, trail_id);
    assert_eq!(added.sequence_number, 1);
    assert_eq!(added.added_by, client.sender_address());
    assert!(added.timestamp > 0);

    let record = records.get(1).await?;
    assert_eq!(record.sequence_number, 1);
    assert_eq!(record.metadata, Some("second metadata".to_string()));
    assert_eq!(record.added_by, client.sender_address());
    assert!(record.added_at > 0);
    assert_text_data(record.data, "second record");

    assert_eq!(records.record_count().await?, 2);

    Ok(())
}

#[tokio::test]
async fn add_and_fetch_tagged_record_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("records-tagged"), ["finance"])
        .await?;
    let records = client.trail(trail_id).records();

    client
        .create_role(
            trail_id,
            "TaggedWriter",
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .issue_cap(trail_id, "TaggedWriter", CapabilityIssueOptions::default())
        .await?;

    let added = records
        .add(
            Data::text("finance record"),
            Some("tagged metadata".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.trail_id, trail_id);
    assert_eq!(added.sequence_number, 1);

    let record = records.get(1).await?;
    assert_eq!(record.tag, Some("finance".to_string()));
    assert_eq!(record.metadata, Some("tagged metadata".to_string()));
    assert_text_data(record.data, "finance record");

    Ok(())
}

#[tokio::test]
async fn add_tagged_record_requires_matching_role_tag_access() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("records-tagged-deny"), ["finance"])
        .await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "PlainWriter", [Permission::AddRecord]).await?;

    let denied = records
        .add(Data::text("should fail"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await;

    assert!(denied.is_err(), "tagged writes should require matching role tag access");
    assert_eq!(records.record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn add_tagged_record_requires_trail_defined_tag() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("records-tagged-undefined"), ["finance"])
        .await?;
    let records = client.trail(trail_id).records();

    client
        .create_role(
            trail_id,
            "TaggedWriter",
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .issue_cap(trail_id, "TaggedWriter", CapabilityIssueOptions::default())
        .await?;

    let denied = records
        .add(Data::text("should fail"), None, Some("legal".to_string()))
        .build_and_execute(&client)
        .await;

    assert!(
        denied.is_err(),
        "tagged writes should require the tag to be defined on the trail"
    );
    assert_eq!(records.record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn add_record_requires_add_record_permission() -> anyhow::Result<()> {
    let admin = get_funded_test_client().await?;
    let writer = get_funded_test_client().await?;
    let trail_id = admin.create_test_trail(Data::text("records-add-permission")).await?;
    let records = writer.trail(trail_id).records();

    admin
        .create_role(trail_id, "NoAddRecord", [Permission::DeleteRecord], None)
        .await?;
    admin
        .issue_cap(
            trail_id,
            "NoAddRecord",
            CapabilityIssueOptions {
                issued_to: Some(writer.sender_address()),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;

    let denied = records
        .add(Data::text("should fail"), None, None)
        .build_and_execute(&writer)
        .await;

    assert!(denied.is_err(), "adding without AddRecord permission must fail");
    assert_eq!(admin.trail(trail_id).records().record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn add_record_selector_skips_revoked_capability_when_valid_one_exists() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;

    // Untagged record flow.
    let trail_id = client.create_test_trail(Data::text("records-revoked-selector")).await?;
    let records = client.trail(trail_id).records();
    let role_name = "RecordWriter";

    client
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;

    // Revoked capability.
    let revoked_cap = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;
    client
        .trail(trail_id)
        .access()
        .revoke_capability(revoked_cap.capability_id, revoked_cap.valid_until)
        .build_and_execute(&client)
        .await?;

    // Valid fallback capability.
    client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

    let added = records
        .add(Data::text("writer record"), None, None)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_text_data(records.get(1).await?.data, "writer record");

    Ok(())
}

#[tokio::test]
async fn revoked_capability_cannot_add_record_without_fallback() -> anyhow::Result<()> {
    let admin = get_funded_test_client().await?;
    let writer = get_funded_test_client().await?;
    let trail_id = admin.create_test_trail(Data::text("records-revoked-hard-fail")).await?;
    let records = writer.trail(trail_id).records();
    let role_name = "RecordWriter";

    admin
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;
    let issued = admin
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                issued_to: Some(writer.sender_address()),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;

    admin
        .trail(trail_id)
        .access()
        .revoke_capability(issued.capability_id, issued.valid_until)
        .build_and_execute(&admin)
        .await?;

    let denied = records
        .add(Data::text("should fail"), None, None)
        .build_and_execute(&writer)
        .await;

    assert!(denied.is_err(), "revoked capabilities must not authorize writes");
    assert_eq!(admin.trail(trail_id).records().record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn add_tagged_record_skips_revoked_capability_when_valid_one_exists() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let tagged_trail_id = client
        .create_test_trail_with_tags(Data::text("records-revoked-tagged"), ["finance"])
        .await?;
    let tagged_records = client.trail(tagged_trail_id).records();
    let tagged_role_name = "TaggedWriter";

    client
        .create_role(
            tagged_trail_id,
            tagged_role_name,
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;

    // Revoked capability.
    let revoked_tagged_cap = client
        .issue_cap(tagged_trail_id, tagged_role_name, CapabilityIssueOptions::default())
        .await?;
    client
        .trail(tagged_trail_id)
        .access()
        .revoke_capability(revoked_tagged_cap.capability_id, revoked_tagged_cap.valid_until)
        .build_and_execute(&client)
        .await?;

    // Valid fallback capability.
    client
        .issue_cap(tagged_trail_id, tagged_role_name, CapabilityIssueOptions::default())
        .await?;

    let tagged_added = tagged_records
        .add(
            Data::text("finance entry"),
            Some("tagged".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(tagged_added.sequence_number, 1);
    assert_eq!(tagged_records.get(1).await?.tag, Some("finance".to_string()));

    Ok(())
}

#[tokio::test]
async fn add_record_selector_skips_expired_capability_when_valid_one_exists() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Untagged record flow.
    let trail_id = client.create_test_trail(Data::text("records-expired-selector")).await?;
    let records = client.trail(trail_id).records();
    let role_name = "RecordWriter";

    client
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;

    // Expired capability.
    client
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                valid_until_ms: Some(now_ms.saturating_sub(60_000)),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;

    // Valid fallback capability.
    client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

    let added = records
        .add(Data::text("writer record"), None, None)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_text_data(records.get(1).await?.data, "writer record");

    // Tagged record flow.
    let tagged_trail_id = client
        .create_test_trail_with_tags(Data::text("records-expired-tagged"), ["finance"])
        .await?;
    let tagged_records = client.trail(tagged_trail_id).records();
    let tagged_role_name = "TaggedWriter";

    client
        .create_role(
            tagged_trail_id,
            tagged_role_name,
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;

    // Expired capability.
    client
        .issue_cap(
            tagged_trail_id,
            tagged_role_name,
            CapabilityIssueOptions {
                valid_until_ms: Some(now_ms.saturating_sub(60_000)),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;

    // Valid fallback capability.
    client
        .issue_cap(tagged_trail_id, tagged_role_name, CapabilityIssueOptions::default())
        .await?;

    let tagged_added = tagged_records
        .add(
            Data::text("finance entry"),
            Some("tagged".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(tagged_added.sequence_number, 1);
    assert_eq!(tagged_records.get(1).await?.tag, Some("finance".to_string()));

    Ok(())
}

#[tokio::test]
async fn add_record_using_capability_uses_selected_capability_without_fallback() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Untagged record flow.
    let trail_id = client
        .create_test_trail(Data::text("records-explicit-cap-selector"))
        .await?;
    let role_name = "RecordWriter";

    client
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;

    let expired_cap = client
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                valid_until_ms: Some(now_ms.saturating_sub(60_000)),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;
    let valid_cap = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

    let denied = client
        .trail(trail_id)
        .records()
        .using_capability(expired_cap.capability_id)
        .add(Data::text("should fail"), None, None)
        .build_and_execute(&client)
        .await;

    assert!(
        denied.is_err(),
        "explicit capability selection should not fall back when the chosen capability is expired"
    );

    let added = client
        .trail(trail_id)
        .records()
        .using_capability(valid_cap.capability_id)
        .add(Data::text("writer record"), None, None)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_text_data(client.trail(trail_id).records().get(1).await?.data, "writer record");

    // Tagged record flow.
    let tagged_trail_id = client
        .create_test_trail_with_tags(Data::text("records-explicit-cap-tagged"), ["finance"])
        .await?;
    let tagged_role_name = "TaggedWriter";

    client
        .create_role(
            tagged_trail_id,
            tagged_role_name,
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;

    let expired_tagged_cap = client
        .issue_cap(
            tagged_trail_id,
            tagged_role_name,
            CapabilityIssueOptions {
                valid_until_ms: Some(now_ms.saturating_sub(60_000)),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;
    let valid_tagged_cap = client
        .issue_cap(tagged_trail_id, tagged_role_name, CapabilityIssueOptions::default())
        .await?;

    let tagged_denied = client
        .trail(tagged_trail_id)
        .records()
        .using_capability(expired_tagged_cap.capability_id)
        .add(
            Data::text("should fail"),
            Some("tagged".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&client)
        .await;

    assert!(
        tagged_denied.is_err(),
        "tagged writes should also use the explicitly selected capability without fallback"
    );

    let tagged_added = client
        .trail(tagged_trail_id)
        .records()
        .using_capability(valid_tagged_cap.capability_id)
        .add(
            Data::text("finance entry"),
            Some("tagged".to_string()),
            Some("finance".to_string()),
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(tagged_added.sequence_number, 1);
    assert_eq!(
        client.trail(tagged_trail_id).records().get(1).await?.tag,
        Some("finance".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn add_record_respects_valid_from_constraint() -> anyhow::Result<()> {
    let admin = get_funded_test_client().await?;
    let writer = get_funded_test_client().await?;
    let trail_id = admin.create_test_trail(Data::text("records-valid-from")).await?;
    let records = writer.trail(trail_id).records();
    let role_name = "RecordWriter";

    admin
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;
    let valid_from_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64
        + 15_000;
    admin
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                issued_to: Some(writer.sender_address()),
                valid_from_ms: Some(valid_from_ms),
                valid_until_ms: None,
            },
        )
        .await?;

    let denied = records
        .add(Data::text("too early"), None, None)
        .build_and_execute(&writer)
        .await;
    assert!(denied.is_err(), "writes before valid_from must fail");

    sleep(Duration::from_secs(16)).await;

    let added = records
        .add(Data::text("on time"), None, None)
        .build_and_execute(&writer)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_text_data(records.get(1).await?.data, "on time");

    Ok(())
}

#[tokio::test]
async fn add_record_respects_valid_until_constraint() -> anyhow::Result<()> {
    let admin = get_funded_test_client().await?;
    let writer = get_funded_test_client().await?;
    let trail_id = admin.create_test_trail(Data::text("records-valid-until")).await?;
    let records = writer.trail(trail_id).records();
    let role_name = "RecordWriter";

    admin
        .create_role(trail_id, role_name, [Permission::AddRecord], None)
        .await?;
    let valid_until_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64
        + 15_000;
    admin
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                issued_to: Some(writer.sender_address()),
                valid_from_ms: None,
                valid_until_ms: Some(valid_until_ms),
            },
        )
        .await?;

    let added = records
        .add(Data::text("before expiry"), None, None)
        .build_and_execute(&writer)
        .await?
        .output;
    assert_eq!(added.sequence_number, 1);

    sleep(Duration::from_secs(16)).await;

    let denied = records
        .add(Data::text("after expiry"), None, None)
        .build_and_execute(&writer)
        .await;
    assert!(denied.is_err(), "writes after valid_until must fail");

    Ok(())
}

#[tokio::test]
async fn add_record_allows_mixed_data_variants() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("text-trail")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    let added = records
        .add(
            Data::bytes(vec![0xFF, 0x00, 0xAA]),
            Some("binary payload".to_string()),
            None,
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_eq!(records.record_count().await?, 2);
    assert_bytes_data(records.get(1).await?.data, &[0xFF, 0x00, 0xAA]);

    Ok(())
}

#[tokio::test]
async fn add_and_fetch_bytes_record_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::bytes(vec![0x10, 0x20, 0x30])).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    let added = records
        .add(
            Data::bytes(vec![0xFF, 0x00, 0xAA]),
            Some("binary payload".to_string()),
            None,
        )
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(added.sequence_number, 1);
    assert_eq!(records.record_count().await?, 2);

    let record = records.get(1).await?;
    assert_eq!(record.metadata, Some("binary payload".to_string()));
    assert_bytes_data(record.data, &[0xFF, 0x00, 0xAA]);

    Ok(())
}

#[tokio::test]
async fn get_missing_record_fails() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("missing-get")).await?;
    let records = client.trail(trail_id).records();

    let missing = records.get(999).await;
    assert!(missing.is_err(), "reading a missing sequence must fail");

    Ok(())
}

#[tokio::test]
async fn delete_record_removes_entry_and_keeps_sequence_monotonic() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("delete-roundtrip")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "RecordAdmin",
        [Permission::AddRecord, Permission::DeleteRecord],
    )
    .await?;

    let added = records
        .add(Data::text("surviving record"), Some("keep me".to_string()), None)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(added.sequence_number, 1);

    let deleted = records.delete(0).build_and_execute(&client).await?.output;
    assert_eq!(deleted.trail_id, trail_id);
    assert_eq!(deleted.sequence_number, 0);
    assert_eq!(deleted.deleted_by, client.sender_address());
    assert!(deleted.timestamp > 0);

    assert_eq!(records.record_count().await?, 1);
    assert!(records.get(0).await.is_err(), "deleted record should be gone");

    let remaining = records.get(1).await?;
    assert_eq!(remaining.sequence_number, 1);
    assert_text_data(remaining.data, "surviving record");

    let on_chain_trail = client.trail(trail_id).get().await?;
    assert_eq!(
        on_chain_trail.sequence_number, 2,
        "sequence_number should stay monotonic even after deletion"
    );

    Ok(())
}

#[tokio::test]
async fn delete_tagged_record_requires_matching_role_tag_access() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("delete-tagged-deny"), ["finance"])
        .await?;

    client
        .create_role(
            trail_id,
            "TaggedWriter",
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .create_role(trail_id, "DeleteOnly", [Permission::DeleteRecord], None)
        .await?;
    client
        .issue_cap(trail_id, "TaggedWriter", CapabilityIssueOptions::default())
        .await?;
    client
        .issue_cap(trail_id, "DeleteOnly", CapabilityIssueOptions::default())
        .await?;

    client
        .trail(trail_id)
        .records()
        .add(Data::text("tagged record"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await?;

    let denied = client
        .trail(trail_id)
        .records()
        .delete(1)
        .build_and_execute(&client)
        .await;

    assert!(
        denied.is_err(),
        "tagged deletes should require matching role tag access"
    );
    assert_eq!(client.trail(trail_id).records().record_count().await?, 2);
    assert_eq!(
        client.trail(trail_id).records().get(1).await?.tag.as_deref(),
        Some("finance")
    );

    Ok(())
}

#[tokio::test]
async fn delete_tagged_record_with_matching_role_tag_access_succeeds() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("delete-tagged-allow"), ["finance"])
        .await?;
    let records = client.trail(trail_id).records();

    client
        .create_role(
            trail_id,
            "TaggedRecordAdmin",
            [Permission::AddRecord, Permission::DeleteRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .issue_cap(trail_id, "TaggedRecordAdmin", CapabilityIssueOptions::default())
        .await?;

    records
        .add(Data::text("tagged record"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await?;

    let deleted = records.delete(1).build_and_execute(&client).await?.output;
    assert_eq!(deleted.sequence_number, 1);
    assert_eq!(records.record_count().await?, 1);
    assert!(records.get(1).await.is_err());

    Ok(())
}

#[tokio::test]
async fn delete_record_requires_delete_permission() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("delete-perm")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "AddOnly", [Permission::AddRecord]).await?;

    let delete_result = records.delete(0).build_and_execute(&client).await;
    assert!(
        delete_result.is_err(),
        "deleting without DeleteRecord permission must fail"
    );
    assert!(records.get(0).await.is_ok(), "record should still exist");

    Ok(())
}

#[tokio::test]
async fn delete_record_not_found_fails() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("delete-not-found")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "DeleteOnly", [Permission::DeleteRecord]).await?;

    let delete_missing = records.delete(999).build_and_execute(&client).await;
    assert!(delete_missing.is_err(), "deleting a non-existent sequence should fail");

    Ok(())
}

#[tokio::test]
async fn delete_record_fails_while_time_locked() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let created = client
        .create_trail()
        .with_initial_record(InitialRecord::new(Data::text("locked"), None, None))
        .with_locking_config(config_with_window(LockingWindow::TimeBased { seconds: 3600 }))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;
    let trail_id = created.trail_id;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "DeleteOnly", [Permission::DeleteRecord]).await?;

    let delete_locked = records.delete(0).build_and_execute(&client).await;
    assert!(delete_locked.is_err(), "time-locked record deletion must fail");
    assert_eq!(records.record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn sequence_numbers_do_not_reuse_deleted_slots() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("sequence-stability")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "RecordAdmin",
        [Permission::AddRecord, Permission::DeleteRecord],
    )
    .await?;

    let first_added = records
        .add(Data::text("first added"), None, None)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(first_added.sequence_number, 1);

    records.delete(1).build_and_execute(&client).await?;

    let second_added = records
        .add(Data::text("second added"), None, None)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(
        second_added.sequence_number, 2,
        "new records must not reuse deleted sequence slots"
    );

    assert!(records.get(1).await.is_err(), "deleted sequence should remain absent");
    assert_eq!(records.record_count().await?, 2);
    assert_text_data(records.get(2).await?.data, "second added");

    Ok(())
}

#[tokio::test]
async fn delete_record_fails_while_count_locked() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let created = client
        .create_trail()
        .with_initial_record(InitialRecord::new(Data::text("count-locked"), None, None))
        .with_locking_config(config_with_window(LockingWindow::CountBased { count: 5 }))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;
    let trail_id = created.trail_id;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "DeleteOnly", [Permission::DeleteRecord]).await?;

    let delete_locked = records.delete(0).build_and_execute(&client).await;
    assert!(delete_locked.is_err(), "count-locked record deletion must fail");
    assert_eq!(records.record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn delete_records_batch_respects_limit_and_deletes_oldest_first() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let created = client
        .create_trail()
        .with_initial_record(InitialRecord::new(Data::text("batch-initial"), None, None))
        .with_locking_config(config_with_window(LockingWindow::TimeBased { seconds: 3600 }))
        .finish()
        .build_and_execute(&client)
        .await?
        .output;

    let trail_id = created.trail_id;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "BatchRecordAdmin",
        [Permission::AddRecord, Permission::DeleteAllRecords],
    )
    .await?;

    records
        .add(Data::text("batch-second"), None, None)
        .build_and_execute(&client)
        .await?;
    records
        .add(Data::text("batch-third"), None, None)
        .build_and_execute(&client)
        .await?;

    assert_eq!(records.record_count().await?, 3);

    let deleted_two = records.delete_records_batch(2).build_and_execute(&client).await?.output;
    assert_eq!(
        deleted_two,
        vec![0, 1],
        "batch delete should return the deleted sequence numbers"
    );
    assert_eq!(records.record_count().await?, 1);
    assert!(records.get(0).await.is_err(), "oldest record should be removed first");
    assert!(
        records.get(1).await.is_err(),
        "second oldest record should also be removed"
    );
    assert_text_data(records.get(2).await?.data, "batch-third");

    let deleted_last = records
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(deleted_last, vec![2], "remaining record should be deleted");
    assert_eq!(records.record_count().await?, 0);

    let deleted_empty = records
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await?
        .output;
    assert!(
        deleted_empty.is_empty(),
        "deleting from an empty trail should return no sequence numbers"
    );

    Ok(())
}

#[tokio::test]
async fn delete_records_batch_requires_delete_all_records_permission() -> anyhow::Result<()> {
    let admin = get_funded_test_client().await?;
    let operator = get_funded_test_client().await?;
    let trail_id = admin.create_test_trail(Data::text("batch-delete-permission")).await?;
    let records = operator.trail(trail_id).records();

    admin
        .create_role(trail_id, "TrailDeleteOnly", [Permission::DeleteAuditTrail], None)
        .await?;
    admin
        .issue_cap(
            trail_id,
            "TrailDeleteOnly",
            CapabilityIssueOptions {
                issued_to: Some(operator.sender_address()),
                ..CapabilityIssueOptions::default()
            },
        )
        .await?;

    let denied = records.delete_records_batch(10).build_and_execute(&operator).await;
    assert!(
        denied.is_err(),
        "batch deletion must require DeleteAllRecords permission"
    );
    assert_eq!(admin.trail(trail_id).records().record_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn delete_records_batch_requires_matching_role_tag_access() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("batch-delete-tagged-deny"), ["finance"])
        .await?;

    client
        .create_role(
            trail_id,
            "TaggedWriter",
            [Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .create_role(trail_id, "DeleteAllWithoutTags", [Permission::DeleteAllRecords], None)
        .await?;
    client
        .issue_cap(trail_id, "TaggedWriter", CapabilityIssueOptions::default())
        .await?;
    client
        .issue_cap(trail_id, "DeleteAllWithoutTags", CapabilityIssueOptions::default())
        .await?;

    client
        .trail(trail_id)
        .records()
        .add(Data::text("tagged record"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await?;

    let denied = client
        .trail(trail_id)
        .records()
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await;

    assert!(
        denied.is_err(),
        "tagged batch deletes should require matching role tag access"
    );
    assert_eq!(client.trail(trail_id).records().record_count().await?, 2);
    assert_eq!(
        client.trail(trail_id).records().get(1).await?.tag.as_deref(),
        Some("finance")
    );

    Ok(())
}

#[tokio::test]
async fn delete_records_batch_with_matching_role_tag_access_succeeds() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("batch-delete-tagged-allow"), ["finance"])
        .await?;
    let records = client.trail(trail_id).records();

    client
        .create_role(
            trail_id,
            "TaggedDeleteAll",
            [Permission::AddRecord, Permission::DeleteAllRecords],
            Some(RoleTags::new(["finance"])),
        )
        .await?;
    client
        .issue_cap(trail_id, "TaggedDeleteAll", CapabilityIssueOptions::default())
        .await?;

    records
        .add(Data::text("tagged record"), None, Some("finance".to_string()))
        .build_and_execute(&client)
        .await?;

    let deleted = records
        .delete_records_batch(10)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(deleted, vec![0, 1]);
    assert_eq!(records.record_count().await?, 0);
    assert!(records.get(1).await.is_err());

    Ok(())
}

#[tokio::test]
async fn list_and_pagination_support_sparse_sequence_numbers() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("pagination")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "RecordAdmin",
        [Permission::AddRecord, Permission::DeleteRecord],
    )
    .await?;

    records
        .add(Data::text("second"), Some("m2".to_string()), None)
        .build_and_execute(&client)
        .await?;
    records
        .add(Data::text("third"), Some("m3".to_string()), None)
        .build_and_execute(&client)
        .await?;
    records.delete(1).build_and_execute(&client).await?;

    assert_eq!(records.record_count().await?, 2);

    let listed = records.list().await?;
    assert_eq!(listed.len(), 2);
    assert!(listed.contains_key(&0));
    assert!(listed.contains_key(&2));

    let too_small = records.list_with_limit(1).await;
    assert!(too_small.is_err(), "limit below table size should fail");

    let page_1 = records.list_page(None, 1).await?;
    assert_eq!(page_1.records.len(), 1);
    assert!(page_1.records.contains_key(&0));
    assert!(page_1.has_next_page);

    let page_2 = records.list_page(page_1.next_cursor, 1).await?;
    assert_eq!(page_2.records.len(), 1);
    assert!(page_2.records.contains_key(&2));
    assert!(!page_2.has_next_page);
    assert!(page_2.next_cursor.is_none());

    Ok(())
}

#[tokio::test]
async fn list_and_pagination_multi_page_through_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("pagination-multi")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "RecordAdmin",
        [Permission::AddRecord, Permission::DeleteRecord],
    )
    .await?;

    for (idx, label) in ["r1", "r2", "r3", "r4", "r5", "r6"].into_iter().enumerate() {
        records
            .add(
                Data::text(format!("record-{label}")),
                Some(format!("meta-{}", idx + 1)),
                None,
            )
            .build_and_execute(&client)
            .await?;
    }

    // Create sparse keys: 0,1,3,4,6
    records.delete(2).build_and_execute(&client).await?;
    records.delete(5).build_and_execute(&client).await?;

    assert_eq!(records.record_count().await?, 5);

    let list = records.list().await?;
    assert_eq!(list.len(), 5);
    assert!(list.contains_key(&0));
    assert!(list.contains_key(&1));
    assert!(list.contains_key(&3));
    assert!(list.contains_key(&4));
    assert!(list.contains_key(&6));
    assert_text_data(
        list.get(&4).expect("record with key 4 should exist").data.clone(),
        "record-r4",
    );

    let limited = records.list_with_limit(5).await?;
    assert_eq!(limited.len(), 5);
    assert!(records.list_with_limit(4).await.is_err());

    // limit=0 returns no records and keeps the traversal cursor at the starting position.
    let empty_page = records.list_page(None, 0).await?;
    assert!(empty_page.records.is_empty());
    assert!(empty_page.has_next_page);
    assert!(empty_page.next_cursor.is_some());

    let page_1 = records.list_page(None, 2).await?;
    assert_eq!(page_1.records.len(), 2);
    assert_eq!(
        page_1.records.keys().copied().collect::<Vec<_>>(),
        vec![0, 1],
        "page keys should be stable and ordered"
    );
    assert!(page_1.records.contains_key(&0));
    assert!(page_1.records.contains_key(&1));
    assert!(page_1.has_next_page);

    let page_2 = records.list_page(page_1.next_cursor, 2).await?;
    assert_eq!(page_2.records.len(), 2);
    assert_eq!(
        page_2.records.keys().copied().collect::<Vec<_>>(),
        vec![3, 4],
        "page keys should be stable and ordered"
    );
    assert!(page_2.records.contains_key(&3));
    assert!(page_2.records.contains_key(&4));
    assert!(page_2.has_next_page);

    let page_3 = records.list_page(page_2.next_cursor, 2).await?;
    assert_eq!(page_3.records.len(), 1);
    assert!(page_3.records.contains_key(&6));
    assert!(!page_3.has_next_page);
    assert!(page_3.next_cursor.is_none());

    Ok(())
}

#[tokio::test]
async fn list_page_cursor_validation_and_mid_cursor_start() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("pagination-cursor")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(
        &client,
        trail_id,
        "RecordAdmin",
        [Permission::AddRecord, Permission::DeleteRecord],
    )
    .await?;

    for label in ["r1", "r2", "r3", "r4"] {
        records
            .add(Data::text(format!("record-{label}")), None, None)
            .build_and_execute(&client)
            .await?;
    }

    // Existing keys are now 0,1,2,3,4.
    let middle_page = records.list_page(Some(2), 2).await?;
    assert_eq!(middle_page.records.len(), 2);
    assert_eq!(
        middle_page.records.keys().copied().collect::<Vec<_>>(),
        vec![2, 3],
        "page keys should be stable and ordered"
    );
    assert!(middle_page.records.contains_key(&2));
    assert!(middle_page.records.contains_key(&3));
    assert!(middle_page.has_next_page);

    // Cursors that do not exist in the linked-table should fail.
    let invalid_cursor = records.list_page(Some(999), 1).await;
    assert!(invalid_cursor.is_err(), "an invalid cursor should produce an error");

    Ok(())
}

#[tokio::test]
async fn list_page_rejects_limit_above_supported_max() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("pagination-cap")).await?;
    let records = client.trail(trail_id).records();

    let result = records.list_page(None, 1_001).await;

    match result {
        Err(Error::InvalidArgument(message)) => {
            assert!(
                message.contains("exceeds max supported page size"),
                "page-size cap error should be explicit: {message}"
            );
        }
        Err(other) => panic!("expected InvalidArgument for oversized limit, got {other}"),
        Ok(page) => panic!("expected oversized limit error, got page: {page:?}"),
    }

    Ok(())
}
