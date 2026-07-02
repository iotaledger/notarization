// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Proof of Inclusion support for the IOTA Notarization Toolkit.

pub mod error;
pub mod proof;
pub mod target;

pub use error::{Error, Result};
pub use proof::{Proof, ProofVersion, TransactionProof};
pub use target::ProofTargets;
