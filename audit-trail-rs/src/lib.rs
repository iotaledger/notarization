// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(missing_docs, rustdoc::all)]

/// Client wrappers for read-only and signing access to audit trails.
pub mod client;
/// Core handles, builders, transactions, and domain types.
pub mod core;
/// Error types returned by the public API.
pub mod error;
pub(crate) mod iota_interaction_adapter;
pub(crate) mod package;

/// A signing audit-trail client that can build write transactions.
pub use client::full_client::AuditTrailClient;
/// Read-only client types and package override configuration.
pub use client::read_only::{AuditTrailClientReadOnly, PackageOverrides};
/// HTTP utilities to implement the trait [HttpClient](product_common::http_client::HttpClient).
#[cfg(feature = "gas-station")]
pub use product_common::http_client;
