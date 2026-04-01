// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Shared serializable domain types for audit trails.
//!
//! These types mirror the on-chain data model closely enough to deserialize ledger state and
//! events, while still providing a Rust-native API for builders, permission management, and
//! higher-level client flows.

/// On-chain trail metadata types.
pub mod audit_trail;
/// Event payload types emitted by audit-trail transactions.
pub mod event;
/// Locking configuration types.
pub mod locking;
/// Permission and permission-set types.
pub mod permission;
/// Record payload and pagination types.
pub mod record;
/// Role, capability, and role-tag types.
pub mod role_map;

pub use audit_trail::*;
pub use event::*;
pub use locking::*;
pub use permission::*;
pub use record::*;
pub use role_map::*;
