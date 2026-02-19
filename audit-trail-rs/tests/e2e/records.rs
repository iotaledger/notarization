// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::types::{CapabilityIssueOptions, Data, LockingConfig, LockingWindow, Permission};
use audit_trails::error::Error;
use iota_interaction::types::base_types::ObjectID;
use product_common::core_client::CoreClient;

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

fn assert_text_data(data: Data, expected: &str) {
    match data {
        Data::Text(actual) => assert_eq!(actual, expected),
        other => panic!("expected text data, got {other:?}"),
    }
}

#[tokio::test]
async fn add_and_fetch_record_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("records-e2e")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    let added = records
        .add(Data::text("second record"), Some("second metadata".to_string()))
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
async fn add_record_rejects_mismatched_data_type() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("text-trail")).await?;
    let records = client.trail(trail_id).records();

    grant_role_capability(&client, trail_id, "RecordWriter", [Permission::AddRecord]).await?;

    let add_mismatch = records
        .add(Data::bytes(vec![0xFF, 0x00, 0xAA]), Some("binary payload".to_string()))
        .build_and_execute(&client)
        .await;

    assert!(
        add_mismatch.is_err(),
        "adding bytes to a text trail should fail before execution"
    );
    assert_eq!(records.record_count().await?, 1);

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
        .add(Data::text("surviving record"), Some("keep me".to_string()))
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
        .with_initial_record(Data::text("locked"), None)
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::TimeBased { seconds: 3600 },
        })
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
        .add(Data::text("first added"), None)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(first_added.sequence_number, 1);

    records.delete(1).build_and_execute(&client).await?;

    let second_added = records
        .add(Data::text("second added"), None)
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
        .with_initial_record(Data::text("count-locked"), None)
        .with_locking_config(LockingConfig {
            delete_record: LockingWindow::CountBased { count: 5 },
        })
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
        .add(Data::text("second"), Some("m2".to_string()))
        .build_and_execute(&client)
        .await?;
    records
        .add(Data::text("third"), Some("m3".to_string()))
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
            .add(Data::text(format!("record-{label}")), Some(format!("meta-{}", idx + 1)))
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
            .add(Data::text(format!("record-{label}")), None)
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
