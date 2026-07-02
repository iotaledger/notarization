// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::{
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEvents},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents},
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::target::ProofTargets;

/// Proof-format version used for compatibility checks and verifier dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProofVersion(u16);

impl ProofVersion {
    /// Current Proof of Inclusion proof-format version.
    pub const CURRENT: Self = Self(1);

    /// Creates a supported proof-format version.
    pub fn new(version: u16) -> Result<Self> {
        let version = Self(version);
        version.validate()?;
        Ok(version)
    }

    /// Returns the numeric proof-format version.
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Returns an error when this version is not supported.
    pub fn validate(self) -> Result<()> {
        if self == Self::CURRENT {
            Ok(())
        } else {
            Err(Error::UnsupportedProofFormatVersion { version: self.value() })
        }
    }
}

/// Transaction evidence packaged in a Proof of Inclusion envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionProof {
    /// Checkpoint contents including the transaction.
    pub checkpoint_contents: CheckpointContents,
    /// Transaction being authenticated.
    pub transaction: Transaction,
    /// Effects of the transaction being authenticated.
    pub effects: TransactionEffects,
    /// Events of the transaction being authenticated, when present.
    pub events: Option<TransactionEvents>,
}

impl TransactionProof {
    /// Creates transaction proof evidence.
    pub fn new(
        checkpoint_contents: CheckpointContents,
        transaction: Transaction,
        effects: TransactionEffects,
        events: Option<TransactionEvents>,
    ) -> Self {
        Self {
            checkpoint_contents,
            transaction,
            effects,
            events,
        }
    }
}

/// Versioned Proof of Inclusion envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    /// Proof-format version.
    pub version: ProofVersion,
    /// Chain or network identity.
    pub chain: ChainIdentifier,
    /// Target claim authenticated by this proof.
    pub target: ProofTargets,
    /// Certified checkpoint summary.
    pub checkpoint_summary: CertifiedCheckpointSummary,
    /// Transaction evidence for the target.
    pub contents_proof: TransactionProof,
}

impl Proof {
    /// Creates a proof envelope from an explicit target and transaction proof.
    pub fn new(
        chain: ChainIdentifier,
        target: ProofTargets,
        checkpoint_summary: CertifiedCheckpointSummary,
        contents_proof: TransactionProof,
    ) -> Self {
        Self {
            version: ProofVersion::CURRENT,
            chain,
            target,
            checkpoint_summary,
            contents_proof,
        }
    }

    /// Returns the proof-format version.
    pub const fn version(&self) -> ProofVersion {
        self.version
    }

    /// Returns the proof target.
    pub const fn target(&self) -> &ProofTargets {
        &self.target
    }

    /// Serializes this proof envelope as JSON.
    pub fn to_json_vec(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Validates proof-format version.
    pub fn validate(&self) -> Result<()> {
        self.version.validate()
    }
}
