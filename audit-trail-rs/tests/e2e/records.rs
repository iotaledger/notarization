// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::client::get_funded_test_client;
use audit_trails::core::types::Data;

#[tokio::test]
async fn add_and_fetch_record_roundtrip() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let metadata = Some("audit-trail-e2e".to_string());

    let created = client
        .create_trail()
        .with_initial_record(Data::text("audit-trail-e2e"), metadata.clone())
        .finish()?
        .build_and_execute(&client)
        .await?
        .output;

    let trail_id = created.trail_id;
    assert!(
        created.admin_capability_id.is_some(),
        "admin capability id should be returned"
    );

    let output = client
        .trail(trail_id)
        .records()
        .add(Data::text("audit-trail-e2e"), metadata.clone())?
        .build_and_execute(&client)
        .await?;

    let added = output.output;
    assert_eq!(added.trail_id, trail_id);

    let record = client.trail(trail_id).records().get(added.sequence_number).await?;
    assert_eq!(record.sequence_number, added.sequence_number);
    assert_eq!(record.metadata, metadata);
    assert_eq!(record.data, Data::text("audit-trail-e2e"));

    Ok(())
}
