// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Core handles, builders, transactions, and domain types for audit trails.
//!
//! The modules in this namespace make up the main domain-facing API:
//!
//! - [`access`] exposes role and capability management
//! - [`builder`] configures trail creation
//! - [`create`] contains the creation transaction types
//! - [`locking`] manages trail locking rules
//! - [`records`] reads and mutates trail records
//! - [`tags`] manages the trail-owned record-tag registry
//! - [`trail`] provides the high-level typed handle bound to a specific trail
//! - [`types`] contains serializable value types shared across the crate

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
