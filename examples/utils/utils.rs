// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use audit_trail::core::types::{CapabilityIssueOptions, PermissionSet, RoleTags};
use audit_trail::{AuditTrailClient, PackageOverrides};
use iota_sdk::types::base_types::{IotaAddress, ObjectID};
use iota_sdk::{IOTA_LOCAL_NETWORK_URL, IotaClientBuilder};
use notarization::client::{NotarizationClient, NotarizationClientReadOnly};
use product_common::test_utils::{InMemSigner, request_funds};

async fn get_iota_client() -> anyhow::Result<iota_sdk::IotaClient> {
    let api_endpoint = std::env::var("API_ENDPOINT").unwrap_or_else(|_| IOTA_LOCAL_NETWORK_URL.to_string());
    IotaClientBuilder::default()
        .build(&api_endpoint)
        .await
        .map_err(|err| anyhow::anyhow!("failed to connect to network; {}", err))
}

fn get_package_id_from_env(env_var_name: &str) -> anyhow::Result<ObjectID> {
    let value = std::env::var(env_var_name)
        .with_context(|| format!("env variable '{env_var_name}' must be set in order to run the examples"))?;

    value
        .parse()
        .with_context(|| format!("invalid package id in {env_var_name}"))
}

pub async fn get_notarization_read_only_client() -> anyhow::Result<NotarizationClientReadOnly> {
    let iota_client = get_iota_client().await?;

    let package_id = get_package_id_from_env("IOTA_NOTARIZATION_PKG_ID")?;

    NotarizationClientReadOnly::new_with_pkg_id(iota_client, package_id)
        .await
        .context("failed to create a read-only NotarizationClient")
}

pub async fn get_funded_notarization_client() -> Result<NotarizationClient<InMemSigner>, anyhow::Error> {
    let signer = InMemSigner::new();
    let sender_address = signer.get_address().await?;

    request_funds(&sender_address).await?;

    let read_only_client = get_notarization_read_only_client().await?;
    let notarization_client: NotarizationClient<InMemSigner> =
        NotarizationClient::new(read_only_client, signer).await?;

    Ok(notarization_client)
}

pub async fn get_funded_audit_trail_client() -> Result<AuditTrailClient<InMemSigner>, anyhow::Error> {
    let iota_client = get_iota_client().await?;

    let audit_trail_pkg_id = get_package_id_from_env("IOTA_AUDIT_TRAIL_PKG_ID")?;

    let tf_components_pkg_id = get_package_id_from_env("IOTA_TF_COMPONENTS_PKG_ID")?;

    let client = AuditTrailClient::from_iota_client(
        iota_client,
        Some(PackageOverrides {
            audit_trail: Some(audit_trail_pkg_id),
            tf_component: Some(tf_components_pkg_id),
        }),
    )
      .await
      .map_err(|e| anyhow::anyhow!("failed to create AuditTrailClient: {e}"))?;

    let signer = InMemSigner::new();
    let sender_address = signer.get_address().await?;
    request_funds(&sender_address).await?;

    client
        .with_signer(signer)
        .await
        .map_err(|e| anyhow::anyhow!("failed to attach signer to AuditTrailClient: {e}"))
}

pub async fn issue_tagged_record_role(
    client: &AuditTrailClient<InMemSigner>,
    trail_id: ObjectID,
    role_name: &str,
    tag: &str,
    issued_to: IotaAddress,
) -> Result<(), anyhow::Error> {
    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .create(PermissionSet::record_admin_permissions(), Some(RoleTags::new([tag])))
        .build_and_execute(client)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create role '{role_name}': {e}"))?;

    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(issued_to),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(client)
        .await
        .map_err(|e| anyhow::anyhow!("failed to issue capability for role '{role_name}': {e}"))?;

    Ok(())
}
