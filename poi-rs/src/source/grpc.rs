// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_grpc_client::{
    CheckpointResponse, Client as GrpcClient, ReadMask,
    read_mask_fields::{CheckpointResponseField, EpochField, ObjectField, ServiceInfoField, TransactionField},
};
use iota_grpc_types::{proto::TryFromProtoError, v1::transaction::ExecutedTransaction};
use iota_sdk_types::{
    CheckpointContents, CheckpointDigest, ObjectId, SignedCheckpointSummary, SignedTransaction, TransactionDigest,
    Version,
};
use iota_types::{
    committee::{Committee, EpochId},
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEffectsAPI},
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
    transaction::Transaction,
};

use super::{Source, SourceCheckpoint, SourceError, SourceErrorKind, SourceTransaction};
use crate::BoxError;

/// Error returned by native gRPC committee reads.
#[derive(Debug, thiserror::Error)]
#[error("gRPC source failed")]
pub struct GrpcSourceError {
    #[source]
    source: BoxError,
}

impl GrpcSourceError {
    fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[async_trait]
impl Source for GrpcClient {
    type CommitteeError = GrpcSourceError;

    async fn chain_identifier(&self, transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        let service_info = self
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
        let transactions = self
            .get_transactions(
                &[transaction_digest],
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
        let effects = parse_effects(transaction_digest, executed_transaction)?;

        parse_transaction(transaction_digest, executed_transaction, effects).map(Some)
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        let objects = self
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
        let (summary, contents) = parse_checkpoint(transaction_digest, &checkpoint)?;

        Ok(SourceCheckpoint { summary, contents })
    }

    async fn committee(&self, epoch: EpochId) -> Result<Committee, Self::CommitteeError> {
        let epoch_info = self
            .get_epoch(Some(epoch), Some(ReadMask::from(EpochField::COMMITTEE)))
            .await
            .map_err(GrpcSourceError::new)?
            .into_inner();
        let committee = epoch_info.committee().map_err(GrpcSourceError::new)?;

        Ok(committee.into())
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, Self::CommitteeError> {
        self.get_service_info(Some(ReadMask::from(ServiceInfoField::EPOCH)))
            .await
            .map(|response| response.body().epoch)
            .map_err(GrpcSourceError::new)
    }

    async fn epoch_close_summary(
        &self,
        epoch: EpochId,
    ) -> Result<Option<CertifiedCheckpointSummary>, Self::CommitteeError> {
        let epoch_info = self
            .get_epoch(
                Some(epoch),
                Some(ReadMask::from(EpochField::EPOCH_CLOSE_PROOF_CHECKPOINT)),
            )
            .await
            .map_err(GrpcSourceError::new)?
            .into_inner();
        let Some(epoch_close_proof) = epoch_info.epoch_close_proof().map_err(GrpcSourceError::new)? else {
            return Ok(None);
        };
        let checkpoint = epoch_close_proof.checkpoint().map_err(GrpcSourceError::new)?;
        let summary = checkpoint
            .summary
            .as_ref()
            .ok_or_else(|| TryFromProtoError::missing("summary"))
            .map_err(GrpcSourceError::new)?;
        let summary = summary.summary().map_err(GrpcSourceError::new)?;
        let signature = checkpoint
            .signature
            .as_ref()
            .ok_or_else(|| TryFromProtoError::missing("signature"))
            .map_err(GrpcSourceError::new)?;
        let signature = signature.signature().map_err(GrpcSourceError::new)?;
        let signed_summary = SignedCheckpointSummary {
            checkpoint: summary,
            signature,
        };
        let certified_summary = signed_summary.try_into().map_err(GrpcSourceError::new)?;

        Ok(Some(certified_summary))
    }
}

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
        })?;

    Ok((checkpoint_summary, checkpoint_contents))
}

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
    .into();
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
