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

use iota_sdk_types::{CheckpointContents, Event, ObjectReference};
use iota_types::{
    committee::Committee,
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEffectsAPI, TransactionEffectsExt, TransactionEvents},
    event::EventID,
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContentsExt},
    object::Object,
    transaction::Transaction,
};
use serde::{Deserialize, Serialize};

use crate::BoxError;

/// An unsupported proof format version.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("unsupported Proof of Inclusion proof format version: {version}")]
pub struct VersionError {
    /// The unsupported version.
    pub version: u16,
}

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
    /// The proof uses an unsupported wire-format version.
    #[error("proof format version is not supported")]
    Version {
        /// The version error.
        #[source]
        source: VersionError,
    },
    /// The committee signature or checkpoint-contents commitment is invalid.
    #[error("checkpoint summary verification failed")]
    CheckpointSummary {
        /// The checkpoint verification error.
        #[source]
        source: BoxError,
    },
    /// The packaged transaction does not match the transaction digest in its effects.
    #[error("transaction digest does not match the execution digest")]
    TransactionDigestMismatch,
    /// The packaged transaction effects are absent from the authenticated checkpoint contents.
    #[error("transaction digest not found in the checkpoint contents")]
    TransactionNotInCheckpoint,
    /// The packaged events do not match the events digest in the transaction effects.
    #[error("events digest does not match the execution digest")]
    EventsDigestMismatch,
    /// Event claims are present but the proof does not contain transaction events.
    #[error("transaction effects refer to events but event data is missing")]
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
    /// The claimed event differs from the event at the requested transaction-local index.
    #[error("event target contents do not match")]
    EventContentsMismatch,
    /// A claimed object does not compute to its packaged object reference.
    #[error("object target reference does not match the object")]
    ObjectReferenceMismatch,
    /// A claimed object reference is absent from the packaged transaction effects.
    #[error("object target was not found in the transaction effects")]
    ObjectNotFound,
}

/// The format version of a serialized [`Proof`].
///
/// Versions are encoded as unsigned integers. This crate currently supports
/// only [`ProofVersion::CURRENT`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProofVersion(u16);

impl ProofVersion {
    /// The version produced and accepted by this crate.
    pub const CURRENT: Self = Self(1);

    /// Creates a supported proof version.
    ///
    /// # Errors
    ///
    /// Returns [`VersionError`] when `version` is not [`Self::CURRENT`].
    pub fn new(version: u16) -> Result<Self, VersionError> {
        let version = Self(version);
        version.validate()?;
        Ok(version)
    }

    /// Returns the numeric version.
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Checks that this version is supported.
    ///
    /// # Errors
    ///
    /// Returns [`VersionError`] when the value is not [`Self::CURRENT`].
    pub fn validate(self) -> Result<(), VersionError> {
        if self == Self::CURRENT {
            Ok(())
        } else {
            Err(VersionError { version: self.value() })
        }
    }
}

impl TryFrom<u16> for ProofVersion {
    type Error = VersionError;

    fn try_from(version: u16) -> Result<Self, Self::Error> {
        Self::new(version)
    }
}

/// Values whose inclusion is claimed by a [`Proof`].
///
/// Objects and events must belong to the proven transaction.
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ProofTargets {
    /// Objects claimed to have been changed by the transaction.
    pub objects: Vec<(ObjectReference, Object)>,

    /// Events claimed to have been emitted by the transaction.
    pub events: Vec<(EventID, Event)>,
}

impl ProofTargets {
    /// Creates an empty set of claims.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an object claim.
    ///
    /// During verification, `object` must compute to `object_ref`, and
    /// `object_ref` must appear among the objects changed by the proven
    /// transaction.
    pub fn add_object(mut self, object_ref: ObjectReference, object: Object) -> Self {
        self.objects.push((object_ref, object));
        self
    }

    /// Adds an event claim.
    ///
    /// During verification, `event_id` must identify the proven transaction, and
    /// `event` must equal the event at its transaction-local sequence number.
    pub fn add_event(mut self, event_id: EventID, event: Event) -> Self {
        self.events.push((event_id, event));
        self
    }
}

/// The data required to prove that a transaction belongs to a checkpoint.
///
/// The transaction effects link the transaction to `checkpoint_contents` and,
/// when present, commit to `events`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionProof {
    /// The contents of the checkpoint containing the transaction.
    pub checkpoint_contents: CheckpointContents,
    /// The transaction being proven.
    pub transaction: Transaction,
    /// The transaction's execution effects.
    pub effects: TransactionEffects,
    /// Events emitted by the transaction, if any.
    pub events: Option<TransactionEvents>,
}

impl TransactionProof {
    /// Creates transaction proof data.
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

/// Evidence that a transaction is included in a certified checkpoint.
///
/// Every proof contains [`TransactionProof`] and may additionally claim objects
/// or events. Call [`ProofVerifier::verify`] to verify these claims.
///
/// [`Proof::chain`] identifies the network reported by the proof source. It is
/// informational and is not checked during verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    /// The proof format version.
    pub version: ProofVersion,
    /// The network reported by the proof source.
    pub chain: ChainIdentifier,
    /// The values claimed by the proof.
    pub target: ProofTargets,
    /// The certified summary of the checkpoint containing the transaction.
    pub checkpoint_summary: CertifiedCheckpointSummary,
    /// The transaction and its checkpoint inclusion data.
    pub transaction_proof: TransactionProof,
}

impl Proof {
    /// Creates a proof using [`ProofVersion::CURRENT`].
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

    /// Returns the proof format version.
    pub const fn version(&self) -> ProofVersion {
        self.version
    }

    /// Returns the values claimed by the proof.
    pub const fn target(&self) -> &ProofTargets {
        &self.target
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

    /// Checks that the proof format version is supported.
    ///
    /// # Errors
    ///
    /// Returns [`VersionError`] when [`Self::version`] is unsupported.
    pub fn validate(&self) -> Result<(), VersionError> {
        self.version.validate()
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
    /// - the proof uses a supported format version;
    /// - the committee certifies the checkpoint summary;
    /// - the checkpoint contents match the digest in that summary;
    /// - the transaction, effects, and optional events are internally consistent;
    /// - the transaction effects occur in the authenticated checkpoint contents;
    /// - every object and event claim matches the proof data.
    ///
    /// # Errors
    ///
    /// Returns an error if any check fails.
    pub fn verify(&self, proof: &Proof) -> Result<(), VerifyError> {
        proof.validate().map_err(|source| VerifyError {
            kind: VerifyErrorKind::Version { source },
        })?;

        let summary = &proof.checkpoint_summary;
        let contents = Some(&proof.transaction_proof.checkpoint_contents);

        summary
            .verify_with_contents(self.committee, contents)
            .map_err(|source| VerifyError {
                kind: VerifyErrorKind::CheckpointSummary {
                    source: Box::new(source),
                },
            })?;

        self.verify_transaction_proof(summary, &proof.transaction_proof)?;
        self.verify_event_targets(&proof.target, &proof.transaction_proof)?;
        self.verify_object_targets(&proof.target, &proof.transaction_proof)?;

        Ok(())
    }

    /// Checks the transaction-to-effects, effects-to-checkpoint, and effects-to-events links.
    fn verify_transaction_proof(
        &self,
        summary: &CertifiedCheckpointSummary,
        transaction_proof: &TransactionProof,
    ) -> Result<(), VerifyError> {
        let execution_digests = transaction_proof.effects.execution_digests();

        if transaction_proof.transaction.digest() != &execution_digests.transaction {
            return Err(VerifyError {
                kind: VerifyErrorKind::TransactionDigestMismatch,
            });
        }

        let transaction_is_in_checkpoint = transaction_proof
            .checkpoint_contents
            .enumerate_transactions(summary)
            .any(|(_, digests)| digests == execution_digests);

        if !transaction_is_in_checkpoint {
            return Err(VerifyError {
                kind: VerifyErrorKind::TransactionNotInCheckpoint,
            });
        }

        if transaction_proof.effects.events_digest()
            != transaction_proof.events.as_ref().map(|events| events.digest()).as_ref()
        {
            return Err(VerifyError {
                kind: VerifyErrorKind::EventsDigestMismatch,
            });
        }

        Ok(())
    }

    /// Checks each event claim against the proven transaction and its packaged events.
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
        for (event_id, event) in &targets.events {
            if event_id.tx_digest != execution_digests.transaction {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventTransactionMismatch,
                });
            }

            let event_index = event_id.event_seq as usize;
            let Some(actual_event) = events.get(event_index) else {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventSequenceOutOfBounds {
                        sequence: event_id.event_seq,
                    },
                });
            };

            if actual_event != event {
                return Err(VerifyError {
                    kind: VerifyErrorKind::EventContentsMismatch,
                });
            }
        }

        Ok(())
    }

    /// Checks each object claim against its reference and the transaction effects.
    fn verify_object_targets(
        &self,
        targets: &ProofTargets,
        transaction_proof: &TransactionProof,
    ) -> Result<(), VerifyError> {
        if targets.objects.is_empty() {
            return Ok(());
        }

        let changed_objects = transaction_proof.effects.all_changed_objects();
        for (object_ref, object) in &targets.objects {
            if object_ref != &object.as_inner().object_ref() {
                return Err(VerifyError {
                    kind: VerifyErrorKind::ObjectReferenceMismatch,
                });
            }

            changed_objects
                .iter()
                .find(|changed_object_ref| &changed_object_ref.0 == object_ref)
                .ok_or(VerifyError {
                    kind: VerifyErrorKind::ObjectNotFound,
                })?;
        }

        Ok(())
    }
}
