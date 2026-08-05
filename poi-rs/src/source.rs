// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_sdk_types::{CheckpointContents, ObjectId, TransactionDigest, Version};
use iota_types::{
    committee::{Committee, EpochId},
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEvents},
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
    transaction::Transaction,
};

use crate::BoxError;

#[cfg(feature = "native-grpc")]
mod grpc;

/// Error returned when a ledger source cannot provide requested data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// A request to the source failed.
    #[error("source request failed")]
    Request {
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// A response could not be decoded or converted.
    #[error("source returned an invalid response")]
    InvalidResponse {
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
    },
    /// Required source data was omitted.
    #[error("source response is missing required data")]
    MissingData {
        /// Underlying response error.
        #[source]
        source: BoxError,
    },
}

impl SourceError {
    /// Creates an error for a failed source request.
    pub fn request(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Request {
            source: Box::new(source),
        }
    }

    /// Creates an error for an invalid source response.
    pub fn invalid_response(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidResponse {
            source: Box::new(source),
        }
    }

    /// Creates an error for required data omitted from a source response.
    pub fn missing_data(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::MissingData {
            source: Box::new(source),
        }
    }
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

/// Ledger-read boundary used by [`crate::ProofBuilder`] and [`crate::CommitteeResolver`].
///
/// Implementations may fetch evidence from native gRPC, a JavaScript client,
/// archive storage, fixtures, or another source. Proof assembly, target
/// validation, committee authentication, and caching remain outside the source.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Source {
    /// Fetches the genesis-checkpoint digest that identifies the source chain.
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError>;

    /// Fetches and decodes one executed transaction.
    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError>;

    /// Fetches and decodes an object, optionally at an exact version.
    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError>;

    /// Fetches and decodes one certified checkpoint and its contents.
    async fn checkpoint(&self, sequence_number: u64) -> Result<SourceCheckpoint, SourceError>;

    /// Fetches the committee reported for `epoch`.
    async fn committee(&self, epoch: EpochId) -> Result<Committee, SourceError>;

    /// Fetches the current epoch reported by the source.
    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError>;

    /// Fetches the certified checkpoint summary that closed `epoch`.
    async fn epoch_close_summary(&self, epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError>;
}
