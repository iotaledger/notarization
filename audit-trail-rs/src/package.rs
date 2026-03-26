// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Package management for audit trail smart contracts.
//!
//! This module handles package ID resolution and registry management
//! for the audit trail Move contracts.

#![allow(dead_code)]

use std::sync::LazyLock;

use iota_interaction::types::base_types::ObjectID;
use product_common::network_name::NetworkName;
use product_common::package_registry::{Env, PackageRegistry};
use product_common::tf_components_registry;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

use crate::client::PackageOverrides;
use crate::error::Error;

type PackageRegistryLock = RwLockReadGuard<'static, PackageRegistry>;
type PackageRegistryLockMut = RwLockWriteGuard<'static, PackageRegistry>;

/// Global registry for audit trail package information.
static AUDIT_TRAIL_PACKAGE_REGISTRY: LazyLock<RwLock<PackageRegistry>> = LazyLock::new(|| {
    let package_history_json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../audit-trail-move/Move.history.json"
    ));
    RwLock::new(
        PackageRegistry::from_package_history_json_str(package_history_json)
            .expect("Move.history.json exists and it's valid"),
    )
});

/// Runtime overrides for TfComponents package information.
static TF_COMPONENTS_OVERRIDE_REGISTRY: LazyLock<RwLock<PackageRegistry>> =
    LazyLock::new(|| RwLock::new(PackageRegistry::default()));

/// Returns a read lock to the package registry.
pub(crate) async fn audit_trail_package_registry() -> PackageRegistryLock {
    AUDIT_TRAIL_PACKAGE_REGISTRY.read().await
}

/// Attempts to acquire a read lock without blocking.
pub(crate) fn try_audit_trail_package_registry() -> Result<PackageRegistryLock, TryLockError> {
    AUDIT_TRAIL_PACKAGE_REGISTRY.try_read()
}

/// Returns a blocking read lock to the package registry.
pub(crate) fn blocking_audit_trail_registry() -> PackageRegistryLock {
    AUDIT_TRAIL_PACKAGE_REGISTRY.blocking_read()
}

/// Returns a write lock to the package registry.
pub(crate) async fn audit_trail_package_registry_mut() -> PackageRegistryLockMut {
    AUDIT_TRAIL_PACKAGE_REGISTRY.write().await
}

/// Attempts to acquire a write lock without blocking.
pub(crate) fn try_audit_trail_package_registry_mut() -> Result<PackageRegistryLockMut, TryLockError> {
    AUDIT_TRAIL_PACKAGE_REGISTRY.try_write()
}

/// Returns a blocking write lock to the package registry.
pub(crate) fn blocking_audit_trail_registry_mut() -> PackageRegistryLockMut {
    AUDIT_TRAIL_PACKAGE_REGISTRY.blocking_write()
}

pub(crate) async fn tf_components_override_registry_mut() -> PackageRegistryLockMut {
    TF_COMPONENTS_OVERRIDE_REGISTRY.write().await
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedPackageIds {
    pub audit_trail_package_id: ObjectID,
    pub tf_components_package_id: ObjectID,
}

pub(crate) async fn resolve_package_ids(
    network: &NetworkName,
    package_overrides: &PackageOverrides,
) -> Result<(NetworkName, ResolvedPackageIds), Error> {
    let chain_id = network.as_ref().to_string();
    let package_registry = audit_trail_package_registry().await;
    let audit_trail_package_id = package_overrides
        .audit_trail_package_id
        .or_else(|| package_registry.package_id(network))
        .ok_or_else(|| {
            Error::InvalidConfig(format!(
                "no information for a published `audit_trail` package on network {network}; try to use `AuditTrailClientReadOnly::new_with_package_overrides`"
            ))
        })?;
    let resolved_network = match chain_id.as_str() {
        product_common::package_registry::MAINNET_CHAIN_ID => {
            NetworkName::try_from("iota").expect("valid network name")
        }
        _ => package_registry
            .chain_alias(&chain_id)
            .and_then(|alias| NetworkName::try_from(alias).ok())
            .unwrap_or_else(|| network.clone()),
    };

    drop(package_registry);

    let env = Env::new_with_alias(chain_id.clone(), resolved_network.as_ref());
    if let Some(audit_trail_package_id) = package_overrides.audit_trail_package_id {
        audit_trail_package_registry_mut()
            .await
            .insert_env_history(env.clone(), vec![audit_trail_package_id]);
    }
    if let Some(tf_components_package_id) = package_overrides.tf_components_package_id {
        tf_components_override_registry_mut()
            .await
            .insert_env_history(env, vec![tf_components_package_id]);
    }

    let tf_components_package_id = resolve_tf_components_package_id(resolved_network.as_ref()).await.ok_or_else(|| {
        Error::InvalidConfig(format!(
            "no information for a published `TfComponents` package on network {network}; try to use `AuditTrailClientReadOnly::new_with_package_overrides`"
        ))
    })?;

    Ok((
        resolved_network,
        ResolvedPackageIds {
            audit_trail_package_id,
            tf_components_package_id,
        },
    ))
}

pub(crate) async fn resolve_tf_components_package_id(network: &str) -> Option<ObjectID> {
    let override_package_id = TF_COMPONENTS_OVERRIDE_REGISTRY.read().await.package_id(network);
    override_package_id.or_else(|| tf_components_registry::tf_components_package_id(network))
}
