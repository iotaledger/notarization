// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_grpc_client::read_mask_fields::{
    CheckpointResponseField, EpochField, ObjectField, ServiceInfoField, TransactionField,
};
use iota_grpc_client::{Client as GrpcClient, ReadMask};
use iota_grpc_types::proto::TryFromProtoError;
use iota_sdk_types::{
    CheckpointContents, CheckpointDigest, ObjectId, SignedCheckpointSummary, SignedTransaction, TransactionDigest,
    Version,
};
use iota_types::committee::{Committee, EpochId};
use iota_types::digests::ChainIdentifier;
use iota_types::effects::TransactionEffectsAPI;
use iota_types::messages_checkpoint::CertifiedCheckpointSummary;
use iota_types::object::Object;
use iota_types::transaction::Transaction;

use super::{Source, SourceCheckpoint, SourceError, SourceTransaction};

#[async_trait]
impl Source for GrpcClient {
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
        let service_info = self
            .get_service_info(Some(ReadMask::from(ServiceInfoField::CHAIN_ID)))
            .await
            .map_err(SourceError::request)?;
        let chain_identifier = service_info
            .body()
            .chain_identifier()
            .map_err(SourceError::invalid_response)?;

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
            .map_err(SourceError::request)?;
        let Some(executed_transaction) = transactions.into_inner().into_iter().next() else {
            return Ok(None);
        };
        let executed_transaction = match executed_transaction {
            Ok(executed_transaction) => executed_transaction,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(SourceError::request(error)),
        };
        let effects = executed_transaction
            .effects()
            .map_err(SourceError::invalid_response)?
            .effects()
            .map_err(SourceError::invalid_response)?;

        let transaction = executed_transaction
            .transaction()
            .map_err(SourceError::invalid_response)?
            .transaction()
            .map_err(SourceError::invalid_response)?;
        let signatures = executed_transaction
            .signatures()
            .map_err(SourceError::invalid_response)?
            .signatures
            .iter()
            .map(|signature| signature.signature().map_err(SourceError::invalid_response))
            .collect::<Result<Vec<_>, SourceError>>()?;
        let transaction: Transaction = SignedTransaction {
            transaction,
            signatures,
        }
        .into();
        let events = if effects.events_digest().is_some() {
            executed_transaction
                .events()
                .map_err(SourceError::missing_data)?
                .events()
                .map_err(SourceError::invalid_response)
                .map(Some)?
        } else {
            None
        };
        let checkpoint_sequence_number = executed_transaction
            .checkpoint_sequence_number()
            .map_err(SourceError::missing_data)?;

        Ok(Some(SourceTransaction {
            transaction,
            effects,
            events,
            checkpoint_sequence_number,
        }))
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        let objects = self
            .get_objects(&[(object_id, version)], Some(ReadMask::from(ObjectField::BCS)))
            .await
            .map_err(SourceError::request)?;
        let Some(response) = objects.into_inner().into_iter().next() else {
            return Ok(None);
        };
        let response = match response {
            Ok(response) => response,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(SourceError::request(error)),
        };
        let object: Object = response.object().map_err(SourceError::invalid_response)?.into();

        Ok(Some(object))
    }

    async fn checkpoint(&self, sequence_number: u64) -> Result<SourceCheckpoint, SourceError> {
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
            .map_err(SourceError::request)?;
        let summary: CertifiedCheckpointSummary = checkpoint
            .signed_summary()
            .map_err(SourceError::invalid_response)?
            .try_into()
            .map_err(SourceError::invalid_response)?;
        let contents: CheckpointContents = checkpoint
            .contents()
            .map_err(SourceError::invalid_response)?
            .contents()
            .map_err(SourceError::invalid_response)?;

        Ok(SourceCheckpoint { summary, contents })
    }

    async fn committee(&self, epoch: EpochId) -> Result<Committee, SourceError> {
        let epoch_info = self
            .get_epoch(Some(epoch), Some(ReadMask::from(EpochField::COMMITTEE)))
            .await
            .map_err(SourceError::request)?
            .into_inner();
        let committee = epoch_info.committee().map_err(SourceError::invalid_response)?;

        Ok(committee.into())
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
        self.get_service_info(Some(ReadMask::from(ServiceInfoField::EPOCH)))
            .await
            .map(|response| response.body().epoch)
            .map_err(SourceError::request)
    }

    async fn epoch_close_summary(&self, epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
        let epoch_info = self
            .get_epoch(
                Some(epoch),
                Some(ReadMask::from(EpochField::EPOCH_CLOSE_PROOF_CHECKPOINT)),
            )
            .await
            .map_err(SourceError::request)?
            .into_inner();
        let Some(epoch_close_proof) = epoch_info.epoch_close_proof().map_err(SourceError::invalid_response)? else {
            return Ok(None);
        };
        let checkpoint = epoch_close_proof.checkpoint().map_err(SourceError::missing_data)?;
        let summary = checkpoint
            .summary
            .as_ref()
            .ok_or_else(|| TryFromProtoError::missing("summary"))
            .map_err(SourceError::missing_data)?;
        let summary = summary.summary().map_err(SourceError::invalid_response)?;
        let signature = checkpoint
            .signature
            .as_ref()
            .ok_or_else(|| TryFromProtoError::missing("signature"))
            .map_err(SourceError::missing_data)?;
        let signature = signature.signature().map_err(SourceError::invalid_response)?;
        let signed_summary = SignedCheckpointSummary {
            checkpoint: summary,
            signature,
        };
        let certified_summary = signed_summary.try_into().map_err(SourceError::invalid_response)?;

        Ok(Some(certified_summary))
    }
}
