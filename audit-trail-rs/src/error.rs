// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Error types returned by the audit-trail public API.

use crate::iota_interaction_adapter::AdapterError;

/// Errors that can occur when reading or mutating audit trails.
#[derive(Debug, thiserror::Error, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum Error {
    /// Returned when a signer key or public key cannot be derived or validated.
    #[error("invalid key: {0}")]
    InvalidKey(String),
    /// Returned when client configuration or package-ID configuration is invalid.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    /// Returned when an RPC request fails.
    #[error("RPC error: {0}")]
    RpcError(String),
    /// Error returned by the underlying IOTA client adapter.
    #[error("IOTA client error: {0}")]
    IotaClient(#[from] AdapterError),
    /// Generic catch-all error for crate-specific failures that do not fit a narrower variant.
    #[error("{0}")]
    GenericError(String),
    /// Placeholder for unimplemented API surface.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    /// Returned when a Move tag cannot be parsed.
    #[error("Failed to parse tag: {0}")]
    FailedToParseTag(String),
    /// Returned when an argument is semantically invalid.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    /// The response from the IOTA node API was not in the expected format.
    #[error("unexpected API response: {0}")]
    UnexpectedApiResponse(String),
    /// Failed to deserialize data using BCS.
    #[error("BCS deserialization error: {0}")]
    DeserializationError(#[from] bcs::Error),
    /// The transaction response from the IOTA node API was not in the expected format.
    #[error("unexpected transaction response: {0}")]
    TransactionUnexpectedResponse(String),
}

#[cfg(target_arch = "wasm32")]
use product_common::impl_wasm_error_from;
#[cfg(target_arch = "wasm32")]
impl_wasm_error_from!(Error);
