// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::{
    committee::Committee,
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEffectsAPI, TransactionEffectsExt, TransactionEvents},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents, EndOfEpochData},
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
///
/// A transaction proof links one transaction to a certified checkpoint. It carries
/// the checkpoint contents, the transaction, its effects, and the transaction
/// events when the transaction emitted events.
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

/// Proof of Inclusion evidence for targets included in a certified checkpoint.
///
/// The envelope always carries transaction evidence. This keeps the public Proof
/// of Inclusion contract focused on inclusion claims rather than generic
/// checkpoint-only verification.
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
    /// Transaction evidence for the inclusion target.
    pub transaction_proof: TransactionProof,
}

impl Proof {
    /// Creates a proof envelope from an explicit target and transaction proof.
    ///
    /// The constructor sets [`ProofVersion::CURRENT`] automatically.
    pub fn new(
        chain: ChainIdentifier,
        target: ProofTargets,
        checkpoint_summary: CertifiedCheckpointSummary,
        transaction_proof: TransactionProof,
    ) -> Self {
        Self {
            version: ProofVersion::CURRENT,
            chain,
            target,
            checkpoint_summary,
            transaction_proof,
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

/// Offline Proof of Inclusion verifier.
///
/// `ProofVerifier` verifies only the proof material supplied by the caller. It
/// does not fetch data, resolve committees, or trust a node.
#[derive(Clone, Copy, Debug)]
pub struct ProofVerifier<'committee> {
    committee: &'committee Committee,
}

impl<'committee> ProofVerifier<'committee> {
    /// Creates a verifier for proofs certified by `committee`.
    pub const fn new(committee: &'committee Committee) -> Self {
        Self { committee }
    }

    /// Returns the committee used by this verifier.
    pub const fn committee(&self) -> &'committee Committee {
        self.committee
    }

    /// Verifies a Proof of Inclusion.
    ///
    /// The verifier checks the checkpoint summary and all transaction evidence
    /// before authenticating object, event, or committee targets.
    pub fn verify(&self, proof: &Proof) -> Result<()> {
        proof.validate()?;

        let summary = &proof.checkpoint_summary;
        let contents = Some(&proof.transaction_proof.checkpoint_contents);

        summary
            .verify_with_contents(self.committee, contents)
            .map_err(|err| Error::CheckpointSummaryVerification {
                reason: err.to_string(),
            })?;

        self.verify_committee_target(summary, &proof.target)?;
        self.verify_transaction_proof(summary, &proof.transaction_proof)?;
        self.verify_event_targets(&proof.target, &proof.transaction_proof)?;
        self.verify_object_targets(&proof.target, &proof.transaction_proof)?;

        Ok(())
    }

    fn verify_committee_target(&self, summary: &CertifiedCheckpointSummary, targets: &ProofTargets) -> Result<()> {
        let Some(expected_committee) = &targets.committee else {
            return Ok(());
        };

        let Some(EndOfEpochData {
            next_epoch_committee, ..
        }) = &summary.end_of_epoch_data
        else {
            return Err(Error::MissingEndOfEpochCommittee);
        };

        let actual_committee = Committee::new(
            summary.epoch().checked_add(1).ok_or(Error::NextEpochOverflow)?,
            next_epoch_committee.iter().cloned().collect(),
        );

        if actual_committee != *expected_committee {
            return Err(Error::CommitteeMismatch);
        }

        Ok(())
    }

    fn verify_transaction_proof(
        &self,
        summary: &CertifiedCheckpointSummary,
        transaction_proof: &TransactionProof,
    ) -> Result<()> {
        let execution_digests = transaction_proof.effects.execution_digests();
        if transaction_proof.transaction.digest() != &execution_digests.transaction {
            return Err(Error::TransactionDigestMismatch);
        }

        let transaction_is_in_checkpoint = transaction_proof
            .checkpoint_contents
            .enumerate_transactions(summary)
            .any(|(_, digests)| digests == &execution_digests);

        if !transaction_is_in_checkpoint {
            return Err(Error::TransactionNotInCheckpoint);
        }

        if transaction_proof.effects.events_digest()
            != transaction_proof.events.as_ref().map(|events| events.digest()).as_ref()
        {
            return Err(Error::EventsDigestMismatch);
        }

        Ok(())
    }

    fn verify_event_targets(&self, targets: &ProofTargets, transaction_proof: &TransactionProof) -> Result<()> {
        if targets.events.is_empty() {
            return Ok(());
        }

        let Some(events) = &transaction_proof.events else {
            return Err(Error::MissingEvents);
        };

        let execution_digests = transaction_proof.effects.execution_digests();
        for (event_id, event) in &targets.events {
            if event_id.tx_digest != execution_digests.transaction {
                return Err(Error::EventTransactionMismatch);
            }

            let event_index = event_id.event_seq as usize;
            let Some(actual_event) = events.get(event_index) else {
                return Err(Error::EventSequenceOutOfBounds {
                    sequence: event_id.event_seq,
                });
            };

            if actual_event != event {
                return Err(Error::EventContentsMismatch);
            }
        }

        Ok(())
    }

    fn verify_object_targets(&self, targets: &ProofTargets, transaction_proof: &TransactionProof) -> Result<()> {
        if targets.objects.is_empty() {
            return Ok(());
        }

        let changed_objects = transaction_proof.effects.all_changed_objects();
        for (object_ref, object) in &targets.objects {
            if object_ref != &object.compute_object_reference() {
                return Err(Error::ObjectReferenceMismatch);
            }

            changed_objects
                .iter()
                .find(|changed_object_ref| &changed_object_ref.0 == object_ref)
                .ok_or(Error::ObjectNotFound)?;
        }

        Ok(())
    }
}
