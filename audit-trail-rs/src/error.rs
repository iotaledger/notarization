// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::iota_interaction_adapter::AdapterError;

/// Errors that can occur when managing Audit Trails
#[derive(Debug, thiserror::Error, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum Error {
    /// Caused by invalid keys.
    #[error("invalid key: {0}")]
    InvalidKey(String),
    /// Config is invalid.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    /// An error caused by either a connection issue or an invalid RPC call.
    #[error("RPC error: {0}")]
    RpcError(String),
    /// The provided IOTA Client returned an error
    #[error("IOTA client error: {0}")]
    IotaClient(#[from] AdapterError),
    /// Generic error
    #[error("{0}")]
    GenericError(String),
    /// Placeholder for unimplemented API surface.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    /// Failed to parse tag
    #[error("Failed to parse tag: {0}")]
    FailedToParseTag(String),
    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    /// The response from the IOTA node API was not in the expected format.
    #[error("unexpected API response: {0}")]
    UnexpectedApiResponse(String),
    /// Failed to deserialize data using BCS.
    #[error("BCS deserialization error: {0}")]
    DeserializationError(#[from] bcs::Error),
    /// The response from the IOTA node API was not in the expected format.
    #[error("unexpected transaction response: {0}")]
    TransactionUnexpectedResponse(String),
}

#[cfg(target_arch = "wasm32")]
use product_common::impl_wasm_error_from;
#[cfg(target_arch = "wasm32")]
impl_wasm_error_from!(Error);
