// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use async_trait::async_trait;
use iota_grpc_client::{
    CheckpointResponse, Client as GrpcClient, ReadMask,
    read_mask_fields::{CheckpointResponseField, ObjectField, ServiceInfoField, TransactionField},
};
use iota_grpc_types::v1::transaction::ExecutedTransaction;
use iota_sdk_types::{Digest, SignedTransaction};
use iota_types::{
    base_types::ObjectRef,
    digests::{ChainIdentifier, CheckpointDigest, TransactionDigest},
    effects::{TransactionEffects, TransactionEffectsAPI},
    event::EventID,
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents},
    object::Object,
    transaction::Transaction,
};

use crate::{BoxError, Proof, ProofTargets, TransactionProof};

// gRPC fields needed to package a transaction proof.
const TRANSACTION_PROOF_FIELDS: &[&str] = &[
    TransactionField::TRANSACTION_BCS,
    TransactionField::SIGNATURES,
    TransactionField::EFFECTS_BCS,
    TransactionField::EVENTS_DIGEST,
    TransactionField::EVENTS_EVENTS_BCS,
    TransactionField::CHECKPOINT,
];

// gRPC fields needed to package an object target.
const OBJECT_PROOF_FIELDS: &[&str] = &[ObjectField::BCS];

// gRPC fields needed to identify the chain.
const CHAIN_IDENTIFIER_FIELDS: &[&str] = &[ServiceInfoField::CHAIN_ID];

// gRPC fields needed to authenticate checkpoint contents.
const CHECKPOINT_PROOF_FIELDS: &[&str] = &[
    CheckpointResponseField::CHECKPOINT_SUMMARY_BCS,
    CheckpointResponseField::CHECKPOINT_SIGNATURE,
    CheckpointResponseField::CHECKPOINT_CONTENTS_BCS,
];

/// Source target requested by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SourceTarget {
    /// A transaction proof request.
    Transaction(TransactionDigest),
    /// An object proof request.
    Object(ObjectRef),
    /// An event proof request.
    Event(EventID),
}

impl fmt::Display for SourceTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transaction(transaction_digest) => write!(f, "transaction {transaction_digest}"),
            Self::Object(object_ref) => write!(f, "object {object_ref:?}"),
            Self::Event(event_id) => write!(f, "event {event_id:?}"),
        }
    }
}

/// Error returned when a source cannot build a proof.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to build proof for {target}")]
pub struct SourceError {
    /// Target requested from the source.
    pub target: SourceTarget,
    /// Source failure details.
    #[source]
    pub kind: SourceErrorKind,
}

impl SourceError {
    /// Creates a source error for a requested transaction.
    pub fn new(transaction_digest: TransactionDigest, kind: SourceErrorKind) -> Self {
        Self::transaction(transaction_digest, kind)
    }

    /// Creates a source error for a requested transaction.
    pub fn transaction(transaction_digest: TransactionDigest, kind: SourceErrorKind) -> Self {
        Self {
            target: SourceTarget::Transaction(transaction_digest),
            kind,
        }
    }

    /// Creates a source error for a requested object.
    pub fn object(object_ref: ObjectRef, kind: SourceErrorKind) -> Self {
        Self {
            target: SourceTarget::Object(object_ref),
            kind,
        }
    }

    /// Creates a source error for a requested event.
    pub fn event(event_id: EventID, kind: SourceErrorKind) -> Self {
        Self {
            target: SourceTarget::Event(event_id),
            kind,
        }
    }
}

/// Transactions involved when stacked proof targets do not share one owner.
#[derive(Debug)]
pub struct TransactionMismatch {
    /// Transaction selected by the first proof target.
    pub expected: TransactionDigest,
    /// Transaction that owns the conflicting target.
    pub actual: TransactionDigest,
}

impl fmt::Display for TransactionMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "target belongs to transaction {}, expected transaction {}",
            self.actual, self.expected
        )
    }
}

/// Kind of proof source failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SourceErrorKind {
    /// Fetching the chain identifier from the source failed.
    #[error("failed to fetch chain identifier")]
    FetchChainIdentifier {
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// Reading or converting the chain identifier failed.
    #[error("failed to read chain identifier")]
    ChainIdentifier {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Fetching the transaction from the source failed.
    #[error("failed to fetch transaction")]
    FetchTransaction {
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// The source returned no transaction for the requested digest.
    #[error("transaction was not found")]
    TransactionNotFound,
    /// Fetching the object from the source failed.
    #[error("failed to fetch object")]
    FetchObject {
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// The source returned no object for the requested reference.
    #[error("object was not found")]
    ObjectNotFound,
    /// Reading or converting the object failed.
    #[error("failed to read object")]
    Object {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// The returned object does not compute to the requested reference.
    #[error("object reference does not match the requested reference")]
    ObjectReferenceMismatch,
    /// The source could not resolve the requested event.
    #[error("event was not found")]
    EventNotFound,
    /// A requested target belongs to a different transaction than the other targets.
    #[error("{mismatch}")]
    TargetTransactionMismatch {
        /// Conflicting transaction details.
        mismatch: Box<TransactionMismatch>,
    },
    /// The transaction response did not expose a checkpoint sequence number.
    #[error("transaction response is missing checkpoint sequence")]
    MissingCheckpointSequence {
        /// Underlying response error.
        #[source]
        source: BoxError,
    },
    /// Fetching the checkpoint from the source failed.
    #[error("failed to fetch checkpoint {sequence_number}")]
    FetchCheckpoint {
        /// Checkpoint sequence number requested from the source.
        sequence_number: u64,
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// Reading or converting the checkpoint summary failed.
    #[error("failed to read checkpoint summary")]
    CheckpointSummary {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Reading or converting checkpoint contents failed.
    #[error("failed to read checkpoint contents")]
    CheckpointContents {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Reading or converting the signed transaction failed.
    #[error("failed to read signed transaction")]
    Transaction {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Reading transaction signatures failed.
    #[error("failed to read transaction signatures")]
    Signatures {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Reading transaction effects failed.
    #[error("failed to read transaction effects")]
    Effects {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Transaction effects commit to events, but the response did not include events.
    #[error("transaction effects refer to events but event data is missing")]
    MissingEvents {
        /// Underlying response error.
        #[source]
        source: BoxError,
    },
    /// Reading transaction events failed.
    #[error("failed to read transaction events")]
    Events {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
}

/// Source boundary for building Proof of Inclusion envelopes.
///
/// Implementations may fetch data from gRPC, archive storage, fixtures, or any
/// other source. Returned proofs are still untrusted until verified with
/// [`crate::ProofVerifier`].
#[async_trait]
pub trait Source {
    /// Builds one proof for a non-empty set of targets.
    ///
    /// All targets must belong to the same transaction. Implementations should
    /// reuse the shared transaction and checkpoint evidence when constructing
    /// stacked object and event targets.
    async fn proof(&self, targets: &[SourceTarget]) -> Result<Proof, SourceError>;
}

/// Proof source backed by an SDK gRPC client.
///
/// Applications normally construct this source through the network and client
/// convenience constructors on [`crate::ProofBuilder`].
pub struct GrpcSource {
    client: GrpcClient,
}

impl GrpcSource {
    /// Wraps an SDK gRPC client as a Proof of Inclusion source.
    pub(crate) fn new(client: GrpcClient) -> Self {
        Self { client }
    }

    /// Returns the underlying client for endpoint-selection tests.
    #[cfg(test)]
    pub(crate) const fn grpc_client(&self) -> &GrpcClient {
        &self.client
    }

    /// Fetches the genesis-checkpoint digest that identifies the source chain.
    async fn chain_identifier(&self, digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        let service_info = self
            .client
            .get_service_info(Some(ReadMask::from(CHAIN_IDENTIFIER_FIELDS)))
            .await
            .map_err(|source| {
                SourceError::transaction(
                    digest,
                    SourceErrorKind::FetchChainIdentifier {
                        source: Box::new(source),
                    },
                )
            })?;
        let chain_identifier = service_info.body().chain_identifier().map_err(|source| {
            SourceError::transaction(
                digest,
                SourceErrorKind::ChainIdentifier {
                    source: Box::new(source),
                },
            )
        })?;

        Ok(ChainIdentifier::from(CheckpointDigest::new(
            chain_identifier.into_inner(),
        )))
    }

    /// Fetches the executed transaction envelope with the fields needed for inclusion.
    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<ExecutedTransaction, SourceError> {
        let digest = Digest::new(transaction_digest.into_inner());
        let transactions = self
            .client
            .get_transactions(&[digest], Some(ReadMask::from(TRANSACTION_PROOF_FIELDS)))
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchTransaction {
                        source: Box::new(source),
                    },
                )
            })?;

        transactions
            .body()
            .first()
            .cloned()
            .ok_or_else(|| SourceError::transaction(transaction_digest, SourceErrorKind::TransactionNotFound))
    }

    /// Fetches the object contents for an exact object reference.
    async fn get_object(&self, object_ref: ObjectRef) -> Result<Object, SourceError> {
        let objects = self
            .client
            .get_objects(
                &[(object_ref.object_id, Some(object_ref.version))],
                Some(ReadMask::from(OBJECT_PROOF_FIELDS)),
            )
            .await
            .map_err(|source| {
                SourceError::object(
                    object_ref,
                    SourceErrorKind::FetchObject {
                        source: Box::new(source),
                    },
                )
            })?;
        let object: Object = objects
            .body()
            .first()
            .ok_or_else(|| SourceError::object(object_ref, SourceErrorKind::ObjectNotFound))?
            .object()
            .map_err(|source| {
                SourceError::object(
                    object_ref,
                    SourceErrorKind::Object {
                        source: Box::new(source),
                    },
                )
            })?
            .into();

        if object.as_inner().object_ref() != object_ref {
            return Err(SourceError::object(
                object_ref,
                SourceErrorKind::ObjectReferenceMismatch,
            ));
        }

        Ok(object)
    }

    /// Fetches the certified checkpoint summary and contents for an executed transaction.
    async fn get_checkpoint(
        &self,
        transaction_digest: TransactionDigest,
        sequence_number: u64,
    ) -> Result<CheckpointResponse, SourceError> {
        self.client
            .get_checkpoint_by_sequence_number(
                sequence_number,
                Some(ReadMask::from(CHECKPOINT_PROOF_FIELDS)),
                None,
                None,
            )
            .await
            .map(|response| response.into_inner())
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchCheckpoint {
                        sequence_number,
                        source: Box::new(source),
                    },
                )
            })
    }

    /// Selects the transaction shared by all targets or rejects a conflicting target.
    fn ensure_same_transaction(
        selected: &mut Option<TransactionDigest>,
        target: SourceTarget,
        transaction_digest: TransactionDigest,
    ) -> Result<(), SourceError> {
        if let Some(expected) = selected {
            if *expected != transaction_digest {
                return Err(SourceError {
                    target,
                    kind: SourceErrorKind::TargetTransactionMismatch {
                        mismatch: Box::new(TransactionMismatch {
                            expected: *expected,
                            actual: transaction_digest,
                        }),
                    },
                });
            }
        } else {
            *selected = Some(transaction_digest);
        }

        Ok(())
    }

    /// Reads the certified summary and contents from a checkpoint response.
    fn parse_checkpoint(
        transaction_digest: TransactionDigest,
        checkpoint: &CheckpointResponse,
    ) -> Result<(CertifiedCheckpointSummary, CheckpointContents), SourceError> {
        let checkpoint_summary: CertifiedCheckpointSummary = checkpoint
            .signed_summary()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::CheckpointSummary {
                        source: Box::new(source),
                    },
                )
            })?
            .try_into()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::CheckpointSummary {
                        source: Box::new(source),
                    },
                )
            })?;
        let checkpoint_contents: CheckpointContents = checkpoint
            .contents()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::CheckpointContents {
                        source: Box::new(source),
                    },
                )
            })?
            .contents()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::CheckpointContents {
                        source: Box::new(source),
                    },
                )
            })
            .and_then(|contents| {
                contents.try_into().map_err(|source| {
                    SourceError::transaction(
                        transaction_digest,
                        SourceErrorKind::CheckpointContents {
                            source: Box::new(source),
                        },
                    )
                })
            })?;

        Ok((checkpoint_summary, checkpoint_contents))
    }

    /// Builds the transaction evidence committed to by the checkpoint contents.
    fn build_transaction_proof(
        transaction_digest: TransactionDigest,
        executed_transaction: &ExecutedTransaction,
        checkpoint_contents: CheckpointContents,
    ) -> Result<TransactionProof, SourceError> {
        let transaction = executed_transaction
            .transaction()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Transaction {
                        source: Box::new(source),
                    },
                )
            })?
            .transaction()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Transaction {
                        source: Box::new(source),
                    },
                )
            })?;
        let signatures = executed_transaction
            .signatures()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Signatures {
                        source: Box::new(source),
                    },
                )
            })?
            .signatures
            .iter()
            .map(|signature| {
                signature.signature().map_err(|source| {
                    SourceError::transaction(
                        transaction_digest,
                        SourceErrorKind::Signatures {
                            source: Box::new(source),
                        },
                    )
                })
            })
            .collect::<Result<Vec<_>, SourceError>>()?;
        let transaction: Transaction = SignedTransaction {
            transaction,
            signatures,
        }
        .try_into()
        .map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::Transaction {
                    source: Box::new(source),
                },
            )
        })?;
        let effects: TransactionEffects = executed_transaction
            .effects()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Effects {
                        source: Box::new(source),
                    },
                )
            })?
            .effects()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Effects {
                        source: Box::new(source),
                    },
                )
            })?;
        let events = if effects.events_digest().is_some() {
            executed_transaction
                .events()
                .map_err(|source| {
                    SourceError::transaction(
                        transaction_digest,
                        SourceErrorKind::MissingEvents {
                            source: Box::new(source),
                        },
                    )
                })?
                .events()
                .map_err(|source| {
                    SourceError::transaction(
                        transaction_digest,
                        SourceErrorKind::Events {
                            source: Box::new(source),
                        },
                    )
                })
                .map(Some)?
        } else {
            None
        };

        Ok(TransactionProof::new(checkpoint_contents, transaction, effects, events))
    }
}

#[async_trait]
impl Source for GrpcSource {
    async fn proof(&self, targets: &[SourceTarget]) -> Result<Proof, SourceError> {
        let mut selected_transaction = None;
        let mut objects = Vec::new();
        let mut events = Vec::new();

        for target in targets.iter().copied() {
            match target {
                SourceTarget::Transaction(transaction_digest) => {
                    Self::ensure_same_transaction(&mut selected_transaction, target, transaction_digest)?;
                }
                SourceTarget::Object(object_ref) => {
                    let object = self.get_object(object_ref).await?;
                    Self::ensure_same_transaction(&mut selected_transaction, target, object.previous_transaction)?;
                    objects.push((object_ref, object));
                }
                SourceTarget::Event(event_id) => {
                    Self::ensure_same_transaction(&mut selected_transaction, target, event_id.tx_digest)?;
                    events.push(event_id);
                }
            }
        }

        let transaction_digest = selected_transaction.expect("ProofBuilder only calls Source with non-empty targets");
        let executed_transaction = self.get_transaction(transaction_digest).await?;
        let chain_identifier = self.chain_identifier(transaction_digest).await?;
        let checkpoint_sequence_number = executed_transaction.checkpoint_sequence_number().map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::MissingCheckpointSequence {
                    source: Box::new(source),
                },
            )
        })?;
        let checkpoint = self
            .get_checkpoint(transaction_digest, checkpoint_sequence_number)
            .await?;
        let (checkpoint_summary, checkpoint_contents) = Self::parse_checkpoint(transaction_digest, &checkpoint)?;
        let transaction_proof =
            Self::build_transaction_proof(transaction_digest, &executed_transaction, checkpoint_contents)?;
        let mut proof = Proof::new(
            chain_identifier,
            ProofTargets::new(),
            checkpoint_summary,
            transaction_proof,
        );

        for (object_ref, object) in objects {
            proof.target = proof.target.add_object(object_ref, object);
        }

        for event_id in events {
            let event = proof
                .transaction_proof
                .events
                .as_ref()
                .and_then(|events| {
                    usize::try_from(event_id.event_seq)
                        .ok()
                        .and_then(|index| events.get(index))
                })
                .cloned()
                .ok_or_else(|| SourceError::event(event_id, SourceErrorKind::EventNotFound))?;
            proof.target = proof.target.add_event(event_id, event);
        }

        Ok(proof)
    }
}
