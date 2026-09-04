// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::{ObjectId, TransactionDigest, Version};
use iota_types::committee::{Committee, EpochId};
use iota_types::digests::ChainIdentifier;
use iota_types::messages_checkpoint::CertifiedCheckpointSummary;
use iota_types::object::Object;
use poi_rs::{Source, SourceCheckpoint, SourceError, SourceTransaction};

#[derive(Clone)]
pub struct RejectingSource;

#[async_trait]
impl Source for RejectingSource {
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
        unreachable!("rejected transactions do not resolve a chain identifier")
    }

    async fn transaction(
        &self,
        _transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        Err(SourceError::request(std::io::Error::other("transaction rejected")))
    }

    async fn object(&self, _object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
        Ok(None)
    }

    async fn checkpoint(&self, _sequence_number: u64) -> Result<Option<SourceCheckpoint>, SourceError> {
        unreachable!("rejected transactions do not resolve a checkpoint")
    }

    async fn committee(&self, _epoch: EpochId) -> Result<Committee, SourceError> {
        unreachable!("proof-only test source does not resolve committees")
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
        unreachable!("proof-only test source does not resolve the current epoch")
    }

    async fn epoch_close_summary(&self, _epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
        unreachable!("proof-only test source does not resolve epoch-close summaries")
    }
}

#[derive(Clone)]
pub struct RecordingSource {
    source: GrpcClient,
    transactions: Arc<Mutex<Vec<TransactionDigest>>>,
    object_override: Option<Object>,
    omit_checkpoints: bool,
}

impl RecordingSource {
    pub fn new(source: GrpcClient, transactions: Arc<Mutex<Vec<TransactionDigest>>>) -> Self {
        Self {
            source,
            transactions,
            object_override: None,
            omit_checkpoints: false,
        }
    }

    pub fn with_object_override(mut self, object: Object) -> Self {
        self.object_override = Some(object);
        self
    }

    pub fn without_checkpoints(mut self) -> Self {
        self.omit_checkpoints = true;
        self
    }
}

#[async_trait]
impl Source for RecordingSource {
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
        self.source.chain_identifier().await
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        self.transactions
            .lock()
            .expect("recorded transactions lock must not be poisoned")
            .push(transaction_digest);

        self.source.transaction(transaction_digest).await
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        if let Some(object) = &self.object_override {
            return Ok(Some(object.clone()));
        }

        self.source.object(object_id, version).await
    }

    async fn checkpoint(&self, sequence_number: u64) -> Result<Option<SourceCheckpoint>, SourceError> {
        if self.omit_checkpoints {
            return Ok(None);
        }

        self.source.checkpoint(sequence_number).await
    }

    async fn committee(&self, epoch: EpochId) -> Result<Committee, SourceError> {
        self.source.committee(epoch).await
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
        self.source.current_epoch().await
    }

    async fn epoch_close_summary(&self, epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
        self.source.epoch_close_summary(epoch).await
    }
}

#[derive(Clone)]
pub struct MissingSource;

#[async_trait]
impl Source for MissingSource {
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
        unreachable!("missing targets do not resolve a chain identifier")
    }

    async fn transaction(
        &self,
        _transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        Ok(None)
    }

    async fn object(&self, _object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
        Ok(None)
    }

    async fn checkpoint(&self, _sequence_number: u64) -> Result<Option<SourceCheckpoint>, SourceError> {
        unreachable!("missing targets do not resolve a checkpoint")
    }

    async fn committee(&self, _epoch: EpochId) -> Result<Committee, SourceError> {
        unreachable!("proof-only test source does not resolve committees")
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
        unreachable!("proof-only test source does not resolve the current epoch")
    }

    async fn epoch_close_summary(&self, _epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
        unreachable!("proof-only test source does not resolve epoch-close summaries")
    }
}
