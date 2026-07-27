// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use async_trait::async_trait;
#[cfg(feature = "native-grpc")]
use iota_grpc_client::{
    CheckpointResponse, Client as GrpcClient, ReadMask,
    read_mask_fields::{CheckpointResponseField, ObjectField, ServiceInfoField, TransactionField},
};
#[cfg(feature = "native-grpc")]
use iota_grpc_types::v1::transaction::ExecutedTransaction;
#[cfg(feature = "native-grpc")]
use iota_sdk_types::{Digest, SignedTransaction};
use iota_sdk_types::{ObjectId, Version};
#[cfg(feature = "native-grpc")]
use iota_types::{digests::CheckpointDigest, effects::TransactionEffectsAPI};
use iota_types::{
    digests::{ChainIdentifier, TransactionDigest},
    effects::{TransactionEffects, TransactionEvents},
    event::EventID,
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents},
    object::Object,
    transaction::Transaction,
};

use crate::BoxError;

/// Source target requested by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SourceTarget {
    /// A transaction proof request.
    Transaction(TransactionDigest),
    /// An object proof request identified by object ID.
    Object(ObjectId),
    /// An event proof request.
    Event(EventID),
}

impl fmt::Display for SourceTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transaction(transaction_digest) => write!(f, "transaction {transaction_digest}"),
            Self::Object(object_id) => write!(f, "object {object_id}"),
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
    pub fn object(object_id: ObjectId, kind: SourceErrorKind) -> Self {
        Self {
            target: SourceTarget::Object(object_id),
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
    /// The source returned no object for the requested ID.
    #[error("object was not found")]
    ObjectNotFound,
    /// Reading or converting the object failed.
    #[error("failed to read object")]
    Object {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// The returned object does not match the requested ID or transaction effects.
    #[error("object reference does not match the requested object")]
    ObjectReferenceMismatch,
    /// The requested object was not changed by the selected transaction.
    #[error("object was not changed by transaction {transaction_digest}")]
    ObjectNotChangedByTransaction {
        /// Transaction selected by the other proof targets.
        transaction_digest: TransactionDigest,
    },
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

/// Decoded transaction evidence returned by a [`Source`].
///
/// This type contains IOTA domain values rather than transport-specific gRPC or
/// protobuf messages.
pub struct SourceTransaction {
    /// Signed transaction being authenticated.
    pub transaction: Transaction,
    /// Effects produced by executing the transaction.
    pub effects: TransactionEffects,
    /// Events emitted by the transaction, when present.
    pub events: Option<TransactionEvents>,
    /// Sequence number of the checkpoint that includes the transaction.
    pub checkpoint_sequence_number: u64,
}

/// Decoded checkpoint evidence returned by a [`Source`].
///
/// The certified summary authenticates the checkpoint contents used by the
/// transaction proof.
pub struct SourceCheckpoint {
    /// Certified checkpoint summary.
    pub summary: CertifiedCheckpointSummary,
    /// Contents committed to by the checkpoint summary.
    pub contents: CheckpointContents,
}

/// Ledger-read boundary used by [`crate::ProofBuilder`].
///
/// Implementations may fetch evidence from native gRPC, a JavaScript client,
/// archive storage, fixtures, or another source. Proof assembly and target
/// validation remain centralized in [`crate::ProofBuilder`].
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Source {
    /// Fetches the genesis-checkpoint digest that identifies the source chain.
    async fn chain_identifier(&self, transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError>;

    /// Fetches and decodes one executed transaction.
    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError>;

    /// Fetches and decodes an object, optionally at an exact version.
    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError>;

    /// Fetches and decodes one certified checkpoint and its contents.
    async fn checkpoint(
        &self,
        transaction_digest: TransactionDigest,
        sequence_number: u64,
    ) -> Result<SourceCheckpoint, SourceError>;
}

/// Proof source backed by an SDK gRPC client.
///
/// Applications normally construct this source through the network and client
/// convenience constructors on [`crate::ProofBuilder`].
#[cfg(feature = "native-grpc")]
pub struct GrpcSource {
    client: GrpcClient,
}

#[cfg(feature = "native-grpc")]
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

    /// Reads transaction effects before resolving transaction-scoped object IDs.
    fn parse_effects(
        transaction_digest: TransactionDigest,
        executed_transaction: &ExecutedTransaction,
    ) -> Result<TransactionEffects, SourceError> {
        executed_transaction
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
            })
    }

    /// Decodes the transaction evidence needed by the transport-independent builder.
    fn parse_transaction(
        transaction_digest: TransactionDigest,
        executed_transaction: &ExecutedTransaction,
        effects: TransactionEffects,
    ) -> Result<SourceTransaction, SourceError> {
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

        let checkpoint_sequence_number = executed_transaction.checkpoint_sequence_number().map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::MissingCheckpointSequence {
                    source: Box::new(source),
                },
            )
        })?;

        Ok(SourceTransaction {
            transaction,
            effects,
            events,
            checkpoint_sequence_number,
        })
    }
}

#[cfg(feature = "native-grpc")]
#[async_trait]
impl Source for GrpcSource {
    async fn chain_identifier(&self, transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        let service_info = self
            .client
            .get_service_info(Some(ReadMask::from(ServiceInfoField::CHAIN_ID)))
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchChainIdentifier {
                        source: Box::new(source),
                    },
                )
            })?;
        let chain_identifier = service_info.body().chain_identifier().map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::ChainIdentifier {
                    source: Box::new(source),
                },
            )
        })?;

        Ok(ChainIdentifier::from(CheckpointDigest::new(
            chain_identifier.into_inner(),
        )))
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        let digest = Digest::new(transaction_digest.into_inner());
        let transactions = self
            .client
            .get_transactions(
                &[digest],
                Some(ReadMask::from(&[
                    TransactionField::TRANSACTION_BCS,
                    TransactionField::SIGNATURES,
                    TransactionField::EFFECTS_BCS,
                    TransactionField::EVENTS_DIGEST,
                    TransactionField::EVENTS_EVENTS_BCS,
                    TransactionField::CHECKPOINT,
                ])),
            )
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchTransaction {
                        source: Box::new(source),
                    },
                )
            })?;
        let Some(executed_transaction) = transactions.body().first() else {
            return Ok(None);
        };
        let effects = Self::parse_effects(transaction_digest, executed_transaction)?;

        Self::parse_transaction(transaction_digest, executed_transaction, effects).map(Some)
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        let objects = self
            .client
            .get_objects(&[(object_id, version)], Some(ReadMask::from(ObjectField::BCS)))
            .await
            .map_err(|source| {
                SourceError::object(
                    object_id,
                    SourceErrorKind::FetchObject {
                        source: Box::new(source),
                    },
                )
            })?;
        let Some(response) = objects.body().first() else {
            return Ok(None);
        };
        let object: Object = response
            .object()
            .map_err(|source| {
                SourceError::object(
                    object_id,
                    SourceErrorKind::Object {
                        source: Box::new(source),
                    },
                )
            })?
            .into();
        Ok(Some(object))
    }

    async fn checkpoint(
        &self,
        transaction_digest: TransactionDigest,
        sequence_number: u64,
    ) -> Result<SourceCheckpoint, SourceError> {
        let checkpoint = self
            .client
            .get_checkpoint_by_sequence_number(
                sequence_number,
                Some(ReadMask::from(&[
                    CheckpointResponseField::CHECKPOINT_SUMMARY_BCS,
                    CheckpointResponseField::CHECKPOINT_SIGNATURE,
                    CheckpointResponseField::CHECKPOINT_CONTENTS_BCS,
                ])),
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
            })?;
        let (summary, contents) = Self::parse_checkpoint(transaction_digest, &checkpoint)?;

        Ok(SourceCheckpoint { summary, contents })
    }
}
