// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(missing_docs, rustdoc::all)]

/// Proof data types and offline verification.
pub mod proof;
/// Sources for constructing proofs.
pub mod source;
/// Target claims authenticated by a proof.
pub mod target;

pub use proof::{
    Proof, ProofVerifier, ProofVersion, SerializationError, SerializationErrorKind, TransactionProof, VerifyError,
    VerifyErrorKind, VersionError,
};
pub use source::{GrpcSource, Source, SourceError, SourceErrorKind};
pub use target::ProofTargets;
