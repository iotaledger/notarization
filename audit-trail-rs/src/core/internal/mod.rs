// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal helpers used to bridge public audit-trail APIs to low-level IOTA object access and
//! programmable transaction construction.

/// Capability lookup helpers for trail-scoped permission checks.
pub(crate) mod capability;
/// Linked-table decoding helpers for traversing on-chain Move collections.
pub(crate) mod linked_table;
/// Serde adapters for Move collection types that are exposed as standard Rust collections.
pub(crate) mod move_collections;
/// Raw trail fetch and decode helpers.
pub(crate) mod trail;
/// Common programmable-transaction building helpers.
pub(crate) mod tx;
