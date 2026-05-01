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
        .audit_trail
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

    if let Some(audit_trail_package_id) = package_overrides.audit_trail {
        let env = Env::new_with_alias(chain_id.clone(), resolved_network.as_ref());
        audit_trail_package_registry_mut()
            .await
            .insert_env_history(env, vec![audit_trail_package_id]);
    }
    let tf_components_package_id = package_overrides
        .tf_component
        .or_else(|| tf_components_registry::tf_components_package_id(resolved_network.as_ref()))
        .ok_or_else(|| {
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn resolves_tf_components_package_id() {
        let network = NetworkName::try_from("testnet").expect("valid network");
        let registry_package_id = tf_components_registry::tf_components_package_id("testnet")
            .expect("testnet TfComponents package is in the registry");
        let override_package_id = ObjectID::random();

        let (_, registry_resolved_package_ids) = resolve_package_ids(&network, &PackageOverrides::default())
            .await
            .expect("registered package IDs are valid");

        assert_eq!(
            registry_resolved_package_ids.tf_components_package_id,
            registry_package_id
        );

        let (_, resolved_package_ids) = resolve_package_ids(
            &network,
            &PackageOverrides {
                audit_trail: Some(ObjectID::random()),
                tf_component: Some(override_package_id),
            },
        )
        .await
        .expect("explicit package overrides are valid");

        assert_eq!(resolved_package_ids.tf_components_package_id, override_package_id);
    }
}
