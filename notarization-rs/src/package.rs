// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Package management for notarization smart contracts.
//!
//! This module handles package ID resolution and registry management
//! for the notarization Move contracts.

#![allow(dead_code)]

use std::sync::LazyLock;

use product_common::package_registry::PackageRegistry;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

type PackageRegistryLock = RwLockReadGuard<'static, PackageRegistry>;
type PackageRegistryLockMut = RwLockWriteGuard<'static, PackageRegistry>;

/// Global registry for notarization package information.
#[allow(deprecated)] // TODO : Remove after MoveHistoryManager is released with product-core
static NOTARIZATION_PACKAGE_REGISTRY: LazyLock<RwLock<PackageRegistry>> = LazyLock::new(|| {
    let package_history_json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../notarization-move/Move.history.json"
    ));
    RwLock::new(
        PackageRegistry::from_package_history_json_str(package_history_json)
            .expect("Move.history.json exists and it's valid"),
    )
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
