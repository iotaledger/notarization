// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use async_trait::async_trait;
use iota_sdk_types::{CheckpointContents, ObjectId, TransactionDigest, Version};
use iota_types::{
    committee::{Committee, EpochId},
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEvents},
    event::EventID,
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
    transaction::Transaction,
};

use crate::BoxError;

#[cfg(feature = "native-grpc")]
mod grpc;
#[cfg(feature = "native-grpc")]
pub use grpc::GrpcSourceError;

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
    #[error("{actual} does not match expected transaction {expected}")]
    TargetTransactionMismatch {
        /// Transaction selected by the first proof target.
        expected: TransactionDigest,
        /// Transaction that owns the conflicting target.
        actual: TransactionDigest,
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

/// Ledger-read boundary used by [`crate::ProofBuilder`] and [`crate::CommitteeResolver`].
///
/// Implementations may fetch evidence from native gRPC, a JavaScript client,
/// archive storage, fixtures, or another source. Proof assembly, target
/// validation, committee authentication, and caching remain outside the source.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Source {
    /// Error returned when the source cannot provide committee evidence.
    type CommitteeError: std::error::Error + Send + Sync + 'static;

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

    /// Fetches the committee reported for `epoch`.
    async fn committee(&self, epoch: EpochId) -> Result<Committee, Self::CommitteeError>;

    /// Fetches the current epoch reported by the source.
    async fn current_epoch(&self) -> Result<Option<EpochId>, Self::CommitteeError>;

    /// Fetches the certified checkpoint summary that closed `epoch`.
    async fn epoch_close_summary(
        &self,
        epoch: EpochId,
    ) -> Result<Option<CertifiedCheckpointSummary>, Self::CommitteeError>;
}
