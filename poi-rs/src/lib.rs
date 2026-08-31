// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(missing_docs, rustdoc::all)]

/// Shared boxed source error used by the crate's typed errors.
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Proof construction builders.
pub mod builder;
/// Verified committee lineage caches for anchored resolution.
pub mod cache;
/// Convenient source-backed client for proof construction and verification.
pub mod client;
/// Committee resolution for checkpoint verification.
pub mod committee;
/// Proof data types and offline verification.
pub mod proof;
/// Ledger evidence source abstraction.
pub mod source;

pub use builder::{ProofBuilder, ProofBuilderError};
pub use cache::{CommitteeCache, CommitteeCacheError, MemoryCommitteeCache};
pub use client::PoiClient;
pub use committee::{
    CommitteeResolution, CommitteeResolutionError, CommitteeResolutionErrorKind, CommitteeResolver,
    ProofVerificationError,
};
pub use proof::{
    Proof, ProofTargets, ProofV1, ProofVerifier, SerializationError, SerializationErrorKind, TransactionProof,
    VerifiedProof, VerifyError, VerifyErrorKind,
};
pub use source::{Source, SourceCheckpoint, SourceError, SourceTransaction};
