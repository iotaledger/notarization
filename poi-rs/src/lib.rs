// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(missing_docs, rustdoc::all)]

/// Shared boxed source error used by the crate's typed errors.
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Verified committee lineage caches for anchored resolution.
pub mod cache;
/// Committee resolution for checkpoint verification.
pub mod committee;
/// Proof data types and offline verification.
pub mod proof;
/// Sources for constructing proofs.
pub mod source;
/// Target claims authenticated by a proof.
pub mod target;

pub use cache::{CommitteeCache, CommitteeCacheError, MemoryCommitteeCache};
pub use committee::{CommitteeResolutionError, CommitteeResolutionErrorKind, CommitteeResolver};
pub use proof::{
    Proof, ProofVerifier, ProofVersion, SerializationError, SerializationErrorKind, TransactionProof, VerifyError,
    VerifyErrorKind, VersionError,
};
pub use source::{GrpcSource, Source, SourceError, SourceErrorKind, SourceTarget};
pub use target::ProofTargets;
