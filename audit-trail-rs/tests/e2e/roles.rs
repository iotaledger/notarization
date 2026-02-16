// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::client::get_funded_test_client;
use audit_trails::core::types::{CapabilityIssueOptions, Data, Permission, PermissionSet, RoleCreated};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use product_common::core_client::CoreClient;
use std::collections::HashSet;

async fn create_trail(client: &crate::client::TestClient) -> anyhow::Result<ObjectID> {
    let created = client
        .create_trail()
        .with_initial_record(Data::text("roles-e2e"), None)
        .finish()
        .build_and_execute(client)
        .await?
        .output;
    Ok(created.trail_id)
}

async fn create_role_with_permissions(
    client: &crate::client::TestClient,
    trail_id: ObjectID,
    role_name: &str,
    permissions: Vec<Permission>,
) -> anyhow::Result<RoleCreated> {
    let expected_permissions = permissions.iter().copied().collect::<HashSet<_>>();
    let created = client
        .trail(trail_id)
        .roles()
        .for_role(role_name)
        .create(PermissionSet { permissions })
        .build_and_execute(client)
        .await?
        .output;

    assert_eq!(created.trail_id, trail_id);
    assert_eq!(created.role, role_name);
    assert_eq!(created.permissions, expected_permissions);
    assert!(created.timestamp > 0);

    Ok(created)
}

#[tokio::test]
async fn create_role_then_issue_capability_default_options() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "auditor";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;

    let issued = roles
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(issued.target_key, trail_id);
    assert_eq!(issued.role, role_name.to_string());
    assert_eq!(issued.issued_to, None);
    assert_eq!(issued.valid_from, None);
    assert_eq!(issued.valid_until, None);

    Ok(())
}

#[tokio::test]
async fn update_role_permissions_then_issue_capability() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "editor";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;

    let updated = roles
        .for_role(role_name)
        .update_permissions(PermissionSet {
            permissions: vec![Permission::AddRecord, Permission::DeleteRecord],
        })
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(updated.trail_id, trail_id);
    assert_eq!(updated.role, role_name.to_string());
    assert_eq!(
        updated.new_permissions,
        [Permission::AddRecord, Permission::DeleteRecord]
            .into_iter()
            .collect::<HashSet<_>>()
    );
    assert!(updated.timestamp > 0);

    let issued = roles
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(issued.target_key, trail_id);
    assert_eq!(issued.role, role_name.to_string());

    Ok(())
}

#[tokio::test]
async fn delete_role_prevents_new_capability_issuance() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "to-delete";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;
    let deleted = roles
        .for_role(role_name)
        .delete()
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(deleted.trail_id, trail_id);
    assert_eq!(deleted.role, role_name.to_string());
    assert!(deleted.timestamp > 0);

    let issue_tx = roles
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions::default());
    let issue_after_delete = issue_tx.build_and_execute(&client).await;
    assert!(
        issue_after_delete.is_err(),
        "issuing a capability for a deleted role must fail"
    );
    Ok(())
}

#[tokio::test]
async fn issue_capability_with_constraints() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "reviewer";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;

    let issued_to = IotaAddress::random_for_testing_only();
    let constrained = CapabilityIssueOptions {
        issued_to: Some(issued_to),
        valid_from_ms: Some(1_700_000_000_000),
        valid_until_ms: Some(1_700_000_001_000),
    };

    let issued = roles
        .for_role(role_name)
        .issue_capability(constrained.clone())
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(issued.target_key, trail_id);
    assert_eq!(issued.role, role_name.to_string());
    assert_eq!(issued.issued_to, constrained.issued_to);
    assert_eq!(issued.valid_from, constrained.valid_from_ms);
    assert_eq!(issued.valid_until, constrained.valid_until_ms);

    Ok(())
}

#[tokio::test]
async fn revoke_capability_emits_expected_event_data() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "revoker";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;

    let issued = roles
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;

    let revoked = roles
        .revoke_capability(issued.capability_id)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(revoked.target_key, trail_id);
    assert_eq!(revoked.capability_id, issued.capability_id);

    Ok(())
}

#[tokio::test]
async fn destroy_capability_emits_expected_event_data() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();
    let role_name = "destroyer";

    create_role_with_permissions(&client, trail_id, role_name, vec![Permission::AddRecord]).await?;

    let issued_for_destroy = roles
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;

    let destroyed = roles
        .destroy_capability(issued_for_destroy.capability_id)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(destroyed.target_key, trail_id);
    assert_eq!(destroyed.capability_id, issued_for_destroy.capability_id);
    assert_eq!(destroyed.role, role_name.to_string());
    assert_eq!(destroyed.issued_to, None);
    assert_eq!(destroyed.valid_from, None);
    assert_eq!(destroyed.valid_until, None);

    Ok(())
}

#[tokio::test]
async fn destroy_initial_admin_capability_emits_expected_event() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = ObjectID::from(admin_cap_ref.0);

    let destroyed = roles
        .destroy_initial_admin_capability(admin_cap_id)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(destroyed.target_key, trail_id);
    assert_eq!(destroyed.capability_id, admin_cap_id);
    assert_eq!(destroyed.role, "Admin".to_string());
    assert_eq!(destroyed.issued_to, None);
    assert_eq!(destroyed.valid_from, None);
    assert_eq!(destroyed.valid_until, None);

    Ok(())
}

#[tokio::test]
async fn revoke_initial_admin_capability_emits_expected_event() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();

    // Issue a second admin capability so we can use the original to revoke it
    let second_admin = roles
        .for_role("Admin")
        .issue_capability(CapabilityIssueOptions::default())
        .build_and_execute(&client)
        .await?
        .output;

    let revoked = roles
        .revoke_initial_admin_capability(second_admin.capability_id)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(revoked.target_key, trail_id);
    assert_eq!(revoked.capability_id, second_admin.capability_id);

    Ok(())
}

#[tokio::test]
async fn regular_destroy_rejects_initial_admin_capability() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = ObjectID::from(admin_cap_ref.0);

    let result = roles
        .destroy_capability(admin_cap_id)
        .build_and_execute(&client)
        .await;

    assert!(
        result.is_err(),
        "destroying an initial admin cap via regular destroy_capability must fail"
    );

    Ok(())
}

#[tokio::test]
async fn regular_revoke_rejects_initial_admin_capability() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = create_trail(&client).await?;
    let roles = client.trail(trail_id).roles();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = ObjectID::from(admin_cap_ref.0);

    let result = roles
        .revoke_capability(admin_cap_id)
        .build_and_execute(&client)
        .await;

    assert!(
        result.is_err(),
        "revoking an initial admin cap via regular revoke_capability must fail"
    );

    Ok(())
}
