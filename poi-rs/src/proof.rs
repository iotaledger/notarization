// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::{CheckpointContents, EndOfEpochData, Event, ObjectReference};
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

/// Error returned when a proof-format version is not supported.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("unsupported Proof of Inclusion proof format version: {version}")]
pub struct VersionError {
    /// Unsupported proof-format version.
    pub version: u16,
}

/// Error returned when a proof cannot be serialized or deserialized.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to serialize or deserialize Proof of Inclusion proof")]
pub struct SerializationError {
    /// Serialization failure details.
    #[source]
    pub kind: SerializationErrorKind,
}

/// Kind of proof serialization or deserialization failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SerializationErrorKind {
    /// JSON serialization or deserialization failed.
    #[error("json serialization or deserialization failed")]
    Json {
        /// Underlying JSON serialization or deserialization error.
        #[source]
        source: serde_json::Error,
    },
}

/// Error returned when offline proof verification fails.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to verify Proof of Inclusion proof")]
pub struct VerifyError {
    /// Verification failure details.
    #[source]
    pub kind: VerifyErrorKind,
}

/// Kind of offline proof-verification failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VerifyErrorKind {
    /// The proof-format version is not supported.
    #[error("proof format version is not supported")]
    Version {
        /// Unsupported version error.
        #[source]
        source: VersionError,
    },
    /// The checkpoint summary or its contents failed verification.
    #[error("checkpoint summary verification failed")]
    CheckpointSummary {
        /// Underlying checkpoint-verification error.
        #[source]
        source: BoxError,
    },
    /// A committee target was requested but the checkpoint is not an end-of-epoch checkpoint.
    #[error("checkpoint summary does not contain an end-of-epoch committee")]
    MissingEndOfEpochCommittee,
    /// The next epoch value overflowed while checking a committee target.
    #[error("next epoch overflows u64")]
    NextEpochOverflow,
    /// The committee target does not match the checkpoint's next committee.
    #[error("committee target does not match the checkpoint summary")]
    CommitteeMismatch,
    /// Transaction data does not match the transaction digest in the effects.
    #[error("transaction digest does not match the execution digest")]
    TransactionDigestMismatch,
    /// The transaction effects are not included in the checkpoint contents.
    #[error("transaction digest not found in the checkpoint contents")]
    TransactionNotInCheckpoint,
    /// Packaged events do not match the digest recorded in the effects.
    #[error("events digest does not match the execution digest")]
    EventsDigestMismatch,
    /// Event targets require packaged transaction events.
    #[error("transaction effects refer to events but event data is missing")]
    MissingEvents,
    /// The event target belongs to a different transaction.
    #[error("event target does not belong to the transaction")]
    EventTransactionMismatch,
    /// The event target sequence number is outside the packaged event list.
    #[error("event sequence number {sequence} is out of bounds")]
    EventSequenceOutOfBounds {
        /// Requested event sequence.
        sequence: u64,
    },
    /// The packaged event does not match the event target.
    #[error("event target contents do not match")]
    EventContentsMismatch,
    /// The object content does not compute to the requested object reference.
    #[error("object target reference does not match the object")]
    ObjectReferenceMismatch,
    /// The transaction effects do not include the requested object reference.
    #[error("object target was not found in the transaction effects")]
    ObjectNotFound,
}

/// Proof-format version used for compatibility checks and verifier dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProofVersion(u16);

impl ProofVersion {
    /// Current Proof of Inclusion proof-format version.
    pub const CURRENT: Self = Self(1);

    /// Creates a supported proof-format version.
    pub fn new(version: u16) -> Result<Self, VersionError> {
        let version = Self(version);
        version.validate()?;
        Ok(version)
    }

    /// Returns the numeric proof-format version.
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Returns an error when this version is not supported.
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

/// Target claims authenticated by a Proof of Inclusion.
///
/// Object and event targets are authenticated through the transaction evidence in
/// the proof. Committee targets authenticate the next epoch committee recorded in
/// an end-of-epoch checkpoint summary.
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ProofTargets {
    /// Objects that need to be certified.
    pub objects: Vec<(ObjectReference, Object)>,

    /// Events that need to be certified.
    pub events: Vec<(EventID, Event)>,

    /// The next committee being certified.
    pub committee: Option<Committee>,
}

impl ProofTargets {
    /// Creates an empty target set.
    ///
    /// Empty targets are mainly useful while constructing proofs incrementally.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an object target by object reference and object contents.
    ///
    /// Verification checks that the object computes to the supplied reference and
    /// that the transaction effects include the reference.
    pub fn add_object(mut self, object_ref: ObjectReference, object: Object) -> Self {
        self.objects.push((object_ref, object));
        self
    }

    /// Adds an event target by event ID and event contents.
    ///
    /// Verification checks that the event belongs to the transaction and matches
    /// the event stored at the requested event sequence.
    pub fn add_event(mut self, event_id: EventID, event: Event) -> Self {
        self.events.push((event_id, event));
        self
    }

    /// Adds a next-epoch committee target.
    ///
    /// Verification checks that the checkpoint is an end-of-epoch checkpoint and
    /// that its next committee matches the supplied committee.
    pub fn set_committee(mut self, committee: Committee) -> Self {
        self.committee = Some(committee);
        self
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
    pub fn to_json_vec(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(|source| SerializationError {
            kind: SerializationErrorKind::Json { source },
        })
    }

    /// Deserializes a proof envelope from JSON bytes.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, SerializationError> {
        serde_json::from_slice(bytes).map_err(|source| SerializationError {
            kind: SerializationErrorKind::Json { source },
        })
    }

    /// Validates proof-format version.
    pub fn validate(&self) -> Result<(), VersionError> {
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

        self.verify_committee_target(summary, &proof.target)?;
        self.verify_transaction_proof(summary, &proof.transaction_proof)?;
        self.verify_event_targets(&proof.target, &proof.transaction_proof)?;
        self.verify_object_targets(&proof.target, &proof.transaction_proof)?;

        Ok(())
    }

    /// Verifies an optional next-epoch committee target against the authenticated
    /// end-of-epoch data in the checkpoint summary.
    fn verify_committee_target(
        &self,
        summary: &CertifiedCheckpointSummary,
        targets: &ProofTargets,
    ) -> Result<(), VerifyError> {
        let Some(expected_committee) = &targets.committee else {
            return Ok(());
        };

        let Some(EndOfEpochData {
            next_epoch_committee, ..
        }) = &summary.end_of_epoch_data
        else {
            return Err(VerifyError {
                kind: VerifyErrorKind::MissingEndOfEpochCommittee,
            });
        };

        let actual_committee = Committee::from_committee_members(
            summary.epoch().checked_add(1).ok_or(VerifyError {
                kind: VerifyErrorKind::NextEpochOverflow,
            })?,
            next_epoch_committee,
        );

        if actual_committee != *expected_committee {
            return Err(VerifyError {
                kind: VerifyErrorKind::CommitteeMismatch,
            });
        }

        Ok(())
    }

    /// Verifies that the transaction matches its effects, appears in the
    /// authenticated checkpoint contents, and carries the committed events.
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

    /// Verifies that every event target belongs to the proven transaction and
    /// matches the event committed at its transaction-local sequence number.
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

    /// Verifies that every object target computes to its claimed reference and
    /// appears among the objects changed by the proven transaction.
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
