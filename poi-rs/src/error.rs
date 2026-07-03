// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Errors returned by Proof of Inclusion proof-contract operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The proof uses a format version this crate cannot verify.
    #[error("unsupported Proof of Inclusion proof format version: {version}")]
    UnsupportedProofFormatVersion {
        /// Unsupported proof-format version.
        version: u16,
    },
    /// The checkpoint summary or its contents failed verification.
    #[error("checkpoint summary verification failed: {reason}")]
    CheckpointSummaryVerification {
        /// Verification failure details from the underlying IOTA type.
        reason: String,
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
    /// The proof could not be serialized or deserialized.
    #[error("proof serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result alias for Proof of Inclusion operations.
pub type Result<T> = core::result::Result<T, Error>;
