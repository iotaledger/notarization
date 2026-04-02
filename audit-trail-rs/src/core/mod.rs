// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Core handles, builders, transactions, and domain types for audit trails.
//!
//! The modules in this namespace make up the main domain-facing API:
//!
//! - [`crate::core::access`] exposes role and capability management
//! - [`crate::core::builder`] configures trail creation
//! - [`crate::core::create`] contains the creation transaction types
//! - [`crate::core::locking`] manages trail locking rules
//! - [`crate::core::records`] reads and mutates trail records
//! - [`crate::core::tags`] manages the trail-owned record-tag registry
//! - [`crate::core::trail`] provides the high-level typed handle bound to a specific trail
//! - [`crate::core::types`] contains serializable value types shared across the crate

/// Role and capability management APIs.
pub mod access;
/// Builder used to configure trail creation.
pub mod builder;
/// Trail-creation transaction types.
pub mod create;
pub(crate) mod internal;
/// Locking configuration APIs.
pub mod locking;
/// Record read and mutation APIs.
pub mod records;
/// Trail-scoped record-tag management APIs.
pub mod tags;
/// High-level trail handle types.
pub mod trail;
/// Shared domain and event types.
pub mod types;
