// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use audit_trails::core::types::{CapabilityIssueOptions, Data, Permission, PermissionSet, RoleTags};
use iota_interaction::types::base_types::IotaAddress;
use product_common::core_client::CoreClient;

use crate::client::get_funded_test_client;

#[tokio::test]
async fn create_role_then_issue_capability_default_options() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let role_name = "auditor";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let issued = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

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
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();
    let role_name = "editor";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let updated = access
        .for_role(role_name)
        .update_permissions(
            PermissionSet {
                permissions: HashSet::from([Permission::AddRecord, Permission::DeleteRecord]),
            },
            None,
        )
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(updated.trail_id, trail_id);
    assert_eq!(updated.role, role_name.to_string());
    assert_eq!(
        updated.permissions.permissions,
        HashSet::from([Permission::AddRecord, Permission::DeleteRecord])
    );
    assert_eq!(updated.data, None);
    assert_eq!(updated.updated_by, client.sender_address());
    assert!(updated.timestamp > 0);

    let issued = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;
    assert_eq!(issued.target_key, trail_id);
    assert_eq!(issued.role, role_name.to_string());

    Ok(())
}

#[tokio::test]
async fn create_role_rejects_undefined_role_tags() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("roles-undefined-create"), ["legal"])
        .await?;

    let created = client
        .create_role(
            trail_id,
            "tagged-writer",
            vec![Permission::AddRecord],
            Some(RoleTags::new(["finance"])),
        )
        .await;

    assert!(
        created.is_err(),
        "creating a role with tags outside the trail registry must fail"
    );

    Ok(())
}

#[tokio::test]
async fn update_role_permissions_rejects_undefined_role_tags() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client
        .create_test_trail_with_tags(Data::text("roles-undefined-update"), ["legal"])
        .await?;
    let access = client.trail(trail_id).access();
    let role_name = "editor";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let updated = access
        .for_role(role_name)
        .update_permissions(
            PermissionSet {
                permissions: HashSet::from([Permission::AddRecord]),
            },
            Some(RoleTags::new(["finance"])),
        )
        .build_and_execute(&client)
        .await;

    assert!(
        updated.is_err(),
        "updating a role with tags outside the trail registry must fail"
    );

    Ok(())
}

#[tokio::test]
async fn delete_role_prevents_new_capability_issuance() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();
    let role_name = "to-delete";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;
    let deleted = access
        .for_role(role_name)
        .delete()
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(deleted.trail_id, trail_id);
    assert_eq!(deleted.role, role_name.to_string());
    assert_eq!(deleted.deleted_by, client.sender_address());
    assert!(deleted.timestamp > 0);

    let issue_tx = access
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
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let role_name = "reviewer";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let issued_to = IotaAddress::random_for_testing_only();
    let constrained = CapabilityIssueOptions {
        issued_to: Some(issued_to),
        valid_from_ms: Some(1_700_000_000_000),
        valid_until_ms: Some(1_700_000_001_000),
    };

    let issued = client.issue_cap(trail_id, role_name, constrained.clone()).await?;

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
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();
    let role_name = "revoker";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let issued = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

    let revoked = access
        .revoke_capability(issued.capability_id, issued.valid_until)
        .build_and_execute(&client)
        .await?
        .output;
    assert_eq!(revoked.target_key, trail_id);
    assert_eq!(revoked.capability_id, issued.capability_id);
    assert_eq!(revoked.valid_until, 0);

    Ok(())
}

#[tokio::test]
async fn destroy_capability_emits_expected_event_data() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();
    let role_name = "destroyer";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let issued = client
        .issue_cap(trail_id, role_name, CapabilityIssueOptions::default())
        .await?;

    let destroyed = access
        .destroy_capability(issued.capability_id)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(destroyed.target_key, trail_id);
    assert_eq!(destroyed.capability_id, issued.capability_id);
    assert_eq!(destroyed.role, role_name.to_string());
    assert_eq!(destroyed.issued_to, None);
    assert_eq!(destroyed.valid_from, None);
    assert_eq!(destroyed.valid_until, None);

    Ok(())
}

#[tokio::test]
async fn destroy_initial_admin_capability_emits_expected_event() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = admin_cap_ref.0;

    let destroyed = access
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
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;

    // Issue a second admin capability so we can use the original to revoke it
    let second_admin = client
        .issue_cap(trail_id, "Admin", CapabilityIssueOptions::default())
        .await?;

    let access = client.trail(trail_id).access();
    let revoked = access
        .revoke_initial_admin_capability(second_admin.capability_id, second_admin.valid_until)
        .build_and_execute(&client)
        .await?
        .output;

    assert_eq!(revoked.target_key, trail_id);
    assert_eq!(revoked.capability_id, second_admin.capability_id);
    assert_eq!(revoked.valid_until, 0);

    Ok(())
}

#[tokio::test]
async fn regular_destroy_rejects_initial_admin_capability() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = admin_cap_ref.0;

    let result = access.destroy_capability(admin_cap_id).build_and_execute(&client).await;

    assert!(
        result.is_err(),
        "destroying an initial admin cap via regular destroy_capability must fail"
    );

    Ok(())
}

#[tokio::test]
async fn regular_revoke_rejects_initial_admin_capability() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();

    let admin_cap_ref = client.get_cap(client.sender_address(), trail_id).await?;
    let admin_cap_id = admin_cap_ref.0;

    let result = access
        .revoke_capability(admin_cap_id, None)
        .build_and_execute(&client)
        .await;

    assert!(
        result.is_err(),
        "revoking an initial admin cap via regular revoke_capability must fail"
    );

    Ok(())
}

#[tokio::test]
async fn cleanup_revoked_capabilities_removes_expired_entries() -> anyhow::Result<()> {
    let client = get_funded_test_client().await?;
    let trail_id = client.create_test_trail(Data::text("access-e2e")).await?;
    let access = client.trail(trail_id).access();
    let role_name = "cleanup-target";

    client
        .create_role(trail_id, role_name, vec![Permission::AddRecord], None)
        .await?;

    let issued = client
        .issue_cap(
            trail_id,
            role_name,
            CapabilityIssueOptions {
                issued_to: None,
                valid_from_ms: None,
                valid_until_ms: Some(1),
            },
        )
        .await?;

    access
        .revoke_capability(issued.capability_id, issued.valid_until)
        .build_and_execute(&client)
        .await?;

    let trail = client.trail(trail_id);
    let before_cleanup = trail.get().await?;
    assert_eq!(before_cleanup.roles.revoked_capabilities.size, 1);

    access.cleanup_revoked_capabilities().build_and_execute(&client).await?;

    let after_cleanup = trail.get().await?;
    assert_eq!(after_cleanup.roles.revoked_capabilities.size, 0);

    Ok(())
}
