// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Proof types and verification.
//!
//! A [`Proof`] contains a certified checkpoint and the data needed to prove that
//! a transaction, and optionally its objects or events, belong to that
//! checkpoint. [`ProofVerifier`] verifies this data against a caller-provided
//! [`Committee`] without making network requests.
//!
//! [`CertifiedCheckpointSummary`]: iota_types::messages_checkpoint::CertifiedCheckpointSummary
//! [`Committee`]: iota_types::committee::Committee

use iota_sdk_types::{CheckpointContents, TransactionDigest};
use iota_types::committee::Committee;
use iota_types::digests::ChainIdentifier;
use iota_types::effects::{TransactionEffects, TransactionEffectsAPI, TransactionEffectsExt, TransactionEvents};
use iota_types::event::EventID;
use iota_types::messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContentsExt};
use iota_types::object::Object;
use iota_types::transaction::Transaction;
use serde::{Deserialize, Serialize};

use crate::BoxError;

/// An error serializing or deserializing a proof.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to serialize or deserialize Proof of Inclusion proof")]
pub struct SerializationError {
    /// The underlying error.
    #[source]
    pub kind: SerializationErrorKind,
}

/// The cause of a [`SerializationError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SerializationErrorKind {
    /// JSON encoding or decoding failed.
    #[error("json serialization or deserialization failed")]
    Json {
        /// Error reported by `serde_json`.
        #[source]
        source: serde_json::Error,
    },
}

/// An error verifying a proof.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to verify Proof of Inclusion proof")]
pub struct VerifyError {
    /// The reason verification failed.
    #[source]
    pub kind: VerifyErrorKind,
}

/// The cause of a [`VerifyError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VerifyErrorKind {
    /// The committee signature or checkpoint-contents commitment is invalid.
    #[error("checkpoint summary verification failed")]
    CheckpointSummary {
        /// The checkpoint verification error.
        #[source]
        source: BoxError,
    },
    /// The proof does not declare a transaction, object, or event target.
    #[error("proof does not contain a target")]
    MissingTarget,
    /// The selected transaction differs from the transaction packaged in the proof.
    #[error("transaction target does not match the packaged transaction")]
    TransactionTargetMismatch,
    /// The packaged transaction does not match the transaction digest in its effects.
    #[error("transaction digest does not match the execution digest")]
    TransactionDigestMismatch,
    /// The packaged transaction effects are absent from the authenticated checkpoint contents.
    #[error("transaction digest not found in the checkpoint contents")]
    TransactionNotInCheckpoint,
    /// The packaged events do not match the events digest in the transaction effects.
    #[error("events digest does not match the execution digest")]
    EventsDigestMismatch,
    /// Event targets are present but the proof does not contain transaction events.
    #[error("event targets require transaction event data")]
    MissingEvents,
    /// An event claim identifies a transaction other than the one proven by the envelope.
    #[error("event target does not belong to the transaction")]
    EventTransactionMismatch,
    /// An event claim refers to an index outside the packaged transaction events.
    #[error("event sequence number {sequence} is out of bounds")]
    EventSequenceOutOfBounds {
        /// Transaction-local event index requested by the claim.
        sequence: u64,
    },
    /// A claimed object reference is absent from the packaged transaction effects.
    #[error("object target was not found in the transaction effects")]
    ObjectNotFound,
}

/// Values the caller selected for a [`Proof`].
///
/// Objects and events must belong to the proven transaction.
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ProofTargets {
    /// Transaction explicitly selected by the caller.
    pub transaction: Option<TransactionDigest>,

    /// Objects explicitly selected by the caller.
    pub objects: Vec<Object>,

    /// Events explicitly selected by the caller.
    pub events: Vec<EventID>,
}

impl ProofTargets {
    /// Creates an empty set of claims.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the selected transaction.
    pub fn set_transaction(mut self, transaction: TransactionDigest) -> Self {
        self.transaction = Some(transaction);
        self
    }

    /// Adds a selected object.
    pub fn add_object(mut self, object: Object) -> Self {
        self.objects.push(object);
        self
    }

    /// Adds a selected event.
    pub fn add_event(mut self, event_id: EventID) -> Self {
        self.events.push(event_id);
        self
    }

    /// Returns whether no target has been selected.
    pub fn is_empty(&self) -> bool {
        self.transaction.is_none() && self.objects.is_empty() && self.events.is_empty()
    }
}

/// Transaction-specific evidence carried by a [`Proof`].
///
/// The effects identify the transaction in its checkpoint. Event data is
/// included when the proof declares event targets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionProof {
    /// The transaction being proven.
    pub transaction: Transaction,
    /// The transaction's execution effects.
    pub effects: TransactionEffects,
    /// Complete event list included when the proof declares event targets.
    pub events: Option<TransactionEvents>,
}

impl TransactionProof {
    /// Creates transaction proof data.
    pub fn new(
        transaction: Transaction,
        effects: TransactionEffects,
        events: impl Into<Option<TransactionEvents>>,
    ) -> Self {
        Self {
            transaction,
            effects,
            events: events.into(),
        }
    }
}

/// Versioned evidence that a transaction is included in a certified checkpoint.
///
/// The enum is non-exhaustive so future crate versions can add support for new
/// proof formats without making matches in downstream crates source-breaking.
/// Its serialized representation is externally tagged, with the variant name
/// identifying the proof format, for example `{ "ProofV1": { ... } }`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Proof {
    /// A version 1 proof.
    ProofV1(ProofV1),
}

/// Version 1 evidence that a transaction is included in a certified checkpoint.
///
/// [`ProofTargets`] records the values selected by the caller. The checkpoint
/// and transaction proof fields contain the evidence for those targets.
///
/// [`Proof::chain`] identifies the network reported by the proof source. It is
/// informational and is not checked during verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofV1 {
    /// The network reported by the proof source.
    pub chain: ChainIdentifier,
    /// The values selected for this proof.
    pub targets: ProofTargets,
    /// The certified summary of the checkpoint containing the transaction.
    pub checkpoint_summary: CertifiedCheckpointSummary,
    /// Contents committed to by the checkpoint summary.
    pub checkpoint_contents: CheckpointContents,
    /// The transaction and its execution data
    pub transaction_proof: TransactionProof,
}

impl ProofV1 {
    const VERSION: u16 = 1;

    /// Creates a version 1 proof payload.
    pub fn new(
        chain: ChainIdentifier,
        targets: ProofTargets,
        checkpoint_summary: CertifiedCheckpointSummary,
        checkpoint_contents: CheckpointContents,
        transaction_proof: TransactionProof,
    ) -> Self {
        Self {
            chain,
            targets,
            checkpoint_summary,
            checkpoint_contents,
            transaction_proof,
        }
    }
}

impl From<ProofV1> for Proof {
    fn from(proof: ProofV1) -> Self {
        Self::ProofV1(proof)
    }
}

impl Proof {
    /// Returns the proof format version.
    pub const fn version(&self) -> u16 {
        match self {
            Self::ProofV1(_) => ProofV1::VERSION,
        }
    }

    /// Returns the network reported by the proof source.
    pub const fn chain(&self) -> &ChainIdentifier {
        match self {
            Self::ProofV1(proof) => &proof.chain,
        }
    }

    /// Returns the values selected for this proof.
    pub const fn targets(&self) -> &ProofTargets {
        match self {
            Self::ProofV1(proof) => &proof.targets,
        }
    }

    /// Returns the certified checkpoint summary.
    pub const fn checkpoint_summary(&self) -> &CertifiedCheckpointSummary {
        match self {
            Self::ProofV1(proof) => &proof.checkpoint_summary,
        }
    }

    /// Returns the checkpoint contents.
    pub const fn checkpoint_contents(&self) -> &CheckpointContents {
        match self {
            Self::ProofV1(proof) => &proof.checkpoint_contents,
        }
    }

    /// Returns the transaction-specific evidence.
    pub const fn transaction_proof(&self) -> &TransactionProof {
        match self {
            Self::ProofV1(proof) => &proof.transaction_proof,
        }
    }

    /// Serializes the proof as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the proof cannot be serialized.
    pub fn to_json_vec(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(|source| SerializationError {
            kind: SerializationErrorKind::Json { source },
        })
    }

    /// Deserializes a proof from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if `bytes` do not contain a valid JSON representation of
    /// a [`Proof`].
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, SerializationError> {
        serde_json::from_slice(bytes).map_err(|source| SerializationError {
            kind: SerializationErrorKind::Json { source },
        })
    }
}

/// Verifies proofs against a trusted committee.
///
/// Verification is offline. The verifier does not resolve committee history or
/// fetch missing proof data. The caller is responsible for supplying the
/// committee that certified the proof's checkpoint.
///
/// The value of [`Proof::chain`] is not used to select or validate the committee.
#[derive(Clone, Copy, Debug)]
pub struct ProofVerifier<'committee> {
    committee: &'committee Committee,
}

impl<'committee> ProofVerifier<'committee> {
    /// Creates a verifier using `committee` as its trust root.
    pub const fn new(committee: &'committee Committee) -> Self {
        Self { committee }
    }

    /// Returns the committee used to verify checkpoint signatures.
    pub const fn committee(&self) -> &'committee Committee {
        self.committee
    }

    /// Verifies a proof and all of its claims.
    ///
    /// Verification checks that:
    ///
    /// - the committee certifies the checkpoint summary;
    /// - the checkpoint contents match the digest in that summary;
    /// - the transaction, effects, and optional events are internally consistent;
    /// - the transaction effects occur in the authenticated checkpoint contents;
    /// - every selected target matches the authenticated proof data.
    ///
    /// # Errors
    ///
    /// Returns an error if any check fails.
    pub fn verify(&self, proof: &Proof) -> Result<(), VerifyError> {
        match proof {
            Proof::ProofV1(proof) => self.verify_v1(proof),
        }
    }

    /// Verifies a version 1 proof and all of its claims.
    fn verify_v1(&self, proof: &ProofV1) -> Result<(), VerifyError> {
        if proof.targets.is_empty() {
            return Err(VerifyError {
                kind: VerifyErrorKind::MissingTarget,
            });
        }

        let summary = &proof.checkpoint_summary;
        let contents = Some(&proof.checkpoint_contents);

        summary
            .verify_with_contents(self.committee, contents)
            .map_err(|source| VerifyError {
                kind: VerifyErrorKind::CheckpointSummary {
                    source: Box::new(source),
                },
            })?;

        self.verify_transaction_proof(summary, &proof.checkpoint_contents, &proof.transaction_proof)?;
        self.verify_targets(&proof.targets, &proof.transaction_proof)?;

        Ok(())
    }

    /// Checks the transaction-to-effects, effects-to-checkpoint, and effects-to-events links.
    fn verify_transaction_proof(
        &self,
        summary: &CertifiedCheckpointSummary,
        checkpoint_contents: &CheckpointContents,
        transaction_proof: &TransactionProof,
    ) -> Result<(), VerifyError> {
        let execution_digests = transaction_proof.effects.execution_digests();

        if transaction_proof.transaction.digest() != &execution_digests.transaction {
            return Err(VerifyError {
                kind: VerifyErrorKind::TransactionDigestMismatch,
            });
        }

        let transaction_is_in_checkpoint = checkpoint_contents
            .enumerate_transactions(summary)
            .any(|(_, digests)| digests == execution_digests);

        if !transaction_is_in_checkpoint {
            return Err(VerifyError {
                kind: VerifyErrorKind::TransactionNotInCheckpoint,
            });
        }

        if let Some(events) = &transaction_proof.events {
            if transaction_proof.effects.events_digest() != Some(&events.digest()) {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventsDigestMismatch,
                });
            }
        }

        Ok(())
    }

    /// Checks every declared target against the transaction proof.
    fn verify_targets(&self, targets: &ProofTargets, transaction_proof: &TransactionProof) -> Result<(), VerifyError> {
        let transaction_digest = transaction_proof.effects.execution_digests().transaction;

        if targets.transaction.is_some_and(|target| target != transaction_digest) {
            return Err(VerifyError {
                kind: VerifyErrorKind::TransactionTargetMismatch,
            });
        }

        self.verify_event_targets(targets, transaction_proof)?;
        self.verify_object_targets(targets, transaction_proof)
    }

    /// Checks each event target against the proven transaction and its packaged events.
    fn verify_event_targets(
        &self,
        targets: &ProofTargets,
        transaction_proof: &TransactionProof,
    ) -> Result<(), VerifyError> {
        if targets.events.is_empty() {
            return Ok(());
        }

        let Some(events) = &transaction_proof.events else {
            return Err(VerifyError {
                kind: VerifyErrorKind::MissingEvents,
            });
        };

        let execution_digests = transaction_proof.effects.execution_digests();
        for event_id in &targets.events {
            if event_id.tx_digest != execution_digests.transaction {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventTransactionMismatch,
                });
            }

            let event_index = event_id.event_seq as usize;
            let Some(_) = events.get(event_index) else {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventSequenceOutOfBounds {
                        sequence: event_id.event_seq,
                    },
                });
            };
        }

        Ok(())
    }

    /// Checks each object target against the transaction effects.
    fn verify_object_targets(
        &self,
        targets: &ProofTargets,
        transaction_proof: &TransactionProof,
    ) -> Result<(), VerifyError> {
        if targets.objects.is_empty() {
            return Ok(());
        }

        let changed_objects = transaction_proof.effects.all_changed_objects();
        for object in &targets.objects {
            let object_ref = object.as_inner().object_ref();
            changed_objects
                .iter()
                .find(|changed_object_ref| changed_object_ref.0 == object_ref)
                .ok_or(VerifyError {
                    kind: VerifyErrorKind::ObjectNotFound,
                })?;
        }

        Ok(())
    }
}
