// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Package management for notarization smart contracts.
//!
//! This module handles package ID resolution and registry management
//! for the notarization Move contracts.

#![allow(dead_code)]

use std::sync::LazyLock;

use iota_interaction::types::base_types::ObjectID;
use product_common::core_client::CoreClientReadOnly;
use product_common::package_registry::PackageRegistry;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

use crate::error::Error;

type PackageRegistryLock = RwLockReadGuard<'static, PackageRegistry>;
type PackageRegistryLockMut = RwLockWriteGuard<'static, PackageRegistry>;

/// Global registry for notarization package information.
static NOTARIZATION_PACKAGE_REGISTRY: LazyLock<RwLock<PackageRegistry>> = LazyLock::new(|| {
    let move_lock_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../notarization-move/Move.lock"));
    RwLock::new(PackageRegistry::from_move_lock_content(move_lock_content).expect("Move.lock exists and it's valid"))
});

/// Returns a read lock to the package registry.
pub(crate) async fn notarization_package_registry() -> PackageRegistryLock {
    NOTARIZATION_PACKAGE_REGISTRY.read().await
}

/// Attempts to acquire a read lock without blocking.
pub(crate) fn try_notarization_package_registry() -> Result<PackageRegistryLock, TryLockError> {
    NOTARIZATION_PACKAGE_REGISTRY.try_read()
}

/// Returns a blocking read lock to the package registry.
pub(crate) fn blocking_notarization_registry() -> PackageRegistryLock {
    NOTARIZATION_PACKAGE_REGISTRY.blocking_read()
}

/// Returns a write lock to the package registry.
pub(crate) async fn notarization_package_registry_mut() -> PackageRegistryLockMut {
    NOTARIZATION_PACKAGE_REGISTRY.write().await
}

/// Attempts to acquire a write lock without blocking.
pub(crate) fn try_notarization_package_registry_mut() -> Result<PackageRegistryLockMut, TryLockError> {
    NOTARIZATION_PACKAGE_REGISTRY.try_write()
}

/// Returns a blocking write lock to the package registry.
pub(crate) fn blocking_notarization_registry_mut() -> PackageRegistryLockMut {
    NOTARIZATION_PACKAGE_REGISTRY.blocking_write()
}

/// Returns the package ID for the notarization package.
pub(crate) async fn notarization_package_id<C>(client: &C) -> Result<ObjectID, Error>
where
    C: CoreClientReadOnly,
{
    let network = client.network_name().as_ref();
    notarization_package_registry()
        .await
        .package_id(network)
        .ok_or_else(|| Error::InvalidConfig(format!("cannot find Notarization package ID for network {network}")))
}
