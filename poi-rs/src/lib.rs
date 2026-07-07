// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(missing_docs, rustdoc::all)]

/// Error types returned by proof operations.
pub mod error;
/// Proof data types and offline verification.
pub mod proof;
/// Target claims authenticated by a proof.
pub mod target;

pub use error::{Error, Result};
pub use proof::{Proof, ProofVerifier, ProofVersion, TransactionProof};
pub use target::ProofTargets;
