// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error as StdError;

use async_trait::async_trait;
use iota_grpc_client::{
    CheckpointResponse, Client as GrpcClient, ReadMask,
    read_mask_fields::{CheckpointResponseField, TransactionField},
};
use iota_grpc_types::v1::transaction::ExecutedTransaction;
use iota_sdk_types::{Digest, SignedTransaction};
use iota_types::{
    digests::{ChainIdentifier, TransactionDigest},
    effects::{TransactionEffects, TransactionEffectsAPI},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents},
    transaction::Transaction,
};

use crate::{Proof, ProofTargets, TransactionProof};

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Error returned when a source cannot build a transaction proof.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to build transaction proof for {transaction_digest}")]
pub struct SourceError {
    /// Transaction requested from the source.
    pub transaction_digest: TransactionDigest,
    /// Source failure details.
    #[source]
    pub kind: SourceErrorKind,
}

impl SourceError {
    /// Creates a source error for a requested transaction.
    pub fn new(transaction_digest: TransactionDigest, kind: SourceErrorKind) -> Self {
        Self {
            transaction_digest,
            kind,
        }
    }
}

/// Kind of transaction-proof source failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SourceErrorKind {
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
    /// Builds a transaction proof from source data.
    ///
    /// The returned proof packages the transaction, effects, optional events,
    /// certified checkpoint summary, and checkpoint contents. The transaction
    /// itself is the authenticated claim, so the proof has no additional object,
    /// event, or committee targets.
    async fn transaction(&self, transaction_digest: TransactionDigest) -> Result<Proof, SourceError>;
}

/// gRPC-backed source for transaction proofs.
///
/// `GrpcSource` fetches transaction and checkpoint data from a connected gRPC
/// node and packages it into a [`Proof`]. The node is treated only as a data
/// source: callers still need to verify the returned proof with a trusted
/// committee before trusting any packaged data.
#[derive(Clone)]
pub struct GrpcSource {
    client: GrpcClient,
}

impl GrpcSource {
    /// Creates a gRPC-backed source from an SDK gRPC client.
    pub fn new(client: GrpcClient) -> Self {
        Self { client }
    }

    /// Returns the underlying SDK gRPC client.
    pub const fn grpc_client(&self) -> &GrpcClient {
        &self.client
    }

    /// Fetches the executed transaction envelope with the fields needed for inclusion.
    async fn fetch_executed_transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<ExecutedTransaction, SourceError> {
        let digest = Digest::new(transaction_digest.into_inner());
        let transactions = self
            .client
            .get_transactions(&[digest], Some(ReadMask::from(TRANSACTION_PROOF_FIELDS)))
            .await
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::FetchTransaction {
                    source: Box::new(source),
                },
            })?;

        transactions.body().first().cloned().ok_or(SourceError {
            transaction_digest,
            kind: SourceErrorKind::TransactionNotFound,
        })
    }

    /// Fetches the certified checkpoint summary and contents for an executed transaction.
    async fn fetch_checkpoint_with_contents(
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
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::FetchCheckpoint {
                    sequence_number,
                    source: Box::new(source),
                },
            })
    }
}

#[async_trait]
impl Source for GrpcSource {
    async fn transaction(&self, transaction_digest: TransactionDigest) -> Result<Proof, SourceError> {
        let executed_transaction = self.fetch_executed_transaction(transaction_digest).await?;
        let checkpoint_sequence_number =
            executed_transaction
                .checkpoint_sequence_number()
                .map_err(|source| SourceError {
                    transaction_digest,
                    kind: SourceErrorKind::MissingCheckpointSequence {
                        source: Box::new(source),
                    },
                })?;
        let checkpoint = self
            .fetch_checkpoint_with_contents(transaction_digest, checkpoint_sequence_number)
            .await?;
        let checkpoint_summary: CertifiedCheckpointSummary = checkpoint
            .signed_summary()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::CheckpointSummary {
                    source: Box::new(source),
                },
            })?
            .try_into()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::CheckpointSummary {
                    source: Box::new(source),
                },
            })?;
        let checkpoint_contents: CheckpointContents = checkpoint
            .contents()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::CheckpointContents {
                    source: Box::new(source),
                },
            })?
            .contents()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::CheckpointContents {
                    source: Box::new(source),
                },
            })
            .and_then(|contents| {
                contents.try_into().map_err(|source| SourceError {
                    transaction_digest,
                    kind: SourceErrorKind::CheckpointContents {
                        source: Box::new(source),
                    },
                })
            })?;
        let transaction = executed_transaction
            .transaction()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::Transaction {
                    source: Box::new(source),
                },
            })?
            .transaction()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::Transaction {
                    source: Box::new(source),
                },
            })?;
        let signatures = executed_transaction
            .signatures()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::Signatures {
                    source: Box::new(source),
                },
            })?
            .signatures
            .iter()
            .map(|signature| {
                signature.signature().map_err(|source| SourceError {
                    transaction_digest,
                    kind: SourceErrorKind::Signatures {
                        source: Box::new(source),
                    },
                })
            })
            .collect::<Result<Vec<_>, SourceError>>()?;
        let transaction: Transaction = SignedTransaction {
            transaction,
            signatures,
        }
        .try_into()
        .map_err(|source| SourceError {
            transaction_digest,
            kind: SourceErrorKind::Transaction {
                source: Box::new(source),
            },
        })?;
        let effects: TransactionEffects = executed_transaction
            .effects()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::Effects {
                    source: Box::new(source),
                },
            })?
            .effects()
            .map_err(|source| SourceError {
                transaction_digest,
                kind: SourceErrorKind::Effects {
                    source: Box::new(source),
                },
            })?;
        let events = if effects.events_digest().is_some() {
            executed_transaction
                .events()
                .map_err(|source| SourceError {
                    transaction_digest,
                    kind: SourceErrorKind::MissingEvents {
                        source: Box::new(source),
                    },
                })?
                .events()
                .map_err(|source| SourceError {
                    transaction_digest,
                    kind: SourceErrorKind::Events {
                        source: Box::new(source),
                    },
                })
                .map(Some)?
        } else {
            None
        };

        Ok(Proof::new(
            ChainIdentifier::from(*checkpoint_summary.digest()),
            ProofTargets::new(),
            checkpoint_summary,
            TransactionProof::new(checkpoint_contents, transaction, effects, events),
        ))
    }
}

// Minimum gRPC fields needed to package a transaction proof.
const TRANSACTION_PROOF_FIELDS: &[&str] = &[
    TransactionField::TRANSACTION_BCS,
    TransactionField::SIGNATURES,
    TransactionField::EFFECTS_BCS,
    TransactionField::EVENTS_DIGEST,
    TransactionField::EVENTS_EVENTS_BCS,
    TransactionField::CHECKPOINT,
];

// Minimum gRPC fields needed to authenticate checkpoint contents.
const CHECKPOINT_PROOF_FIELDS: &[&str] = &[
    CheckpointResponseField::CHECKPOINT_SUMMARY_BCS,
    CheckpointResponseField::CHECKPOINT_SIGNATURE,
    CheckpointResponseField::CHECKPOINT_CONTENTS_BCS,
];
