// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Errors returned by Proof of Inclusion proof-contract operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The proof uses a format version this crate cannot verify.
    #[error("unsupported Proof of Inclusion proof format version: {version}")]
    UnsupportedProofFormatVersion {
        /// Unsupported proof-format version.
        version: u16,
    },
    /// The proof could not be serialized or deserialized.
    #[error("proof serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result alias for Proof of Inclusion operations.
pub type Result<T> = core::result::Result<T, Error>;
