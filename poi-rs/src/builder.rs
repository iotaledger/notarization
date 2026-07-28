// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::{ObjectId, ObjectReference, TransactionDigest};
use iota_types::{effects::TransactionEffectsExt, event::EventID, object::Object};

use crate::{Proof, ProofTargets, Source, SourceError, TransactionProof};

/// Ledger target requested from a [`ProofBuilder`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProofTarget {
    /// A transaction proof request.
    Transaction(TransactionDigest),
    /// An object proof request identified by object ID.
    Object(ObjectId),
    /// An event proof request.
    Event(EventID),
}

impl fmt::Display for ProofTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transaction(transaction_digest) => write!(f, "transaction {transaction_digest}"),
            Self::Object(object_id) => write!(f, "object {object_id}"),
            Self::Event(event_id) => write!(f, "event {event_id:?}"),
        }
    }
}

/// Error returned when a proof cannot be constructed by [`ProofBuilder`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofBuilderError {
    /// No proof target was selected before building.
    #[error("proof builder requires a target")]
    MissingTarget,
    /// The configured source failed while reading evidence for a target.
    #[error("source failed while reading {target}")]
    Source {
        /// Proof target whose evidence was being read.
        target: ProofTarget,
        /// Underlying source failure.
        #[source]
        source: SourceError,
    },
    /// The source did not return evidence for a requested target.
    #[error("{target} was not found")]
    TargetNotFound {
        /// Proof target that was not returned.
        target: ProofTarget,
    },
    /// The returned object does not match the requested ID or transaction effects.
    #[error("object {object_id} reference does not match the requested object")]
    ObjectReferenceMismatch {
        /// Requested object ID.
        object_id: ObjectId,
    },
    /// The requested object was not changed by the selected transaction.
    #[error("object {object_id} was not changed by transaction {transaction_digest}")]
    ObjectNotChangedByTransaction {
        /// Requested object ID.
        object_id: ObjectId,
        /// Transaction selected by the other proof targets.
        transaction_digest: TransactionDigest,
    },
    /// A requested target belongs to a different transaction than the other targets.
    #[error("{target} belongs to transaction {actual}, expected {expected}")]
    TargetTransactionMismatch {
        /// Target that conflicts with the previously selected transaction.
        target: ProofTarget,
        /// Transaction selected by the first proof target.
        expected: TransactionDigest,
        /// Transaction that owns the conflicting target.
        actual: TransactionDigest,
    },
}

/// Constructs Proof of Inclusion evidence from a caller-provided [`Source`].
///
/// The builder keeps proof construction independent of a specific transport.
/// With the `native-grpc` feature enabled, SDK gRPC clients can be adapted
/// through `ProofBuilder::from_grpc_client`.
pub struct ProofBuilder<S> {
    source: S,
    targets: Vec<ProofTarget>,
}

#[cfg(feature = "native-grpc")]
impl ProofBuilder<GrpcClient> {
    /// Creates a proof builder connected to the public IOTA mainnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for mainnet.
    pub fn mainnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_mainnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder connected to the public IOTA testnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for testnet.
    pub fn testnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_testnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder connected to the public IOTA devnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for devnet.
    pub fn devnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_devnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder backed by an existing SDK gRPC client.
    pub fn from_grpc_client(client: GrpcClient) -> Self {
        Self::new(client)
    }
}

impl<S: Source> ProofBuilder<S> {
    /// Creates a proof builder backed by `source`.
    pub fn new(source: S) -> Self {
        Self {
            source,
            targets: Vec::new(),
        }
    }

    /// Adds a transaction proof target.
    pub fn transaction(mut self, transaction_digest: TransactionDigest) -> Self {
        self.push_target(ProofTarget::Transaction(transaction_digest));
        self
    }

    /// Adds an object proof target by object ID.
    ///
    /// The source resolves the ID to the exact object reference packaged in the proof.
    pub fn object(mut self, object_id: ObjectId) -> Self {
        self.push_target(ProofTarget::Object(object_id));
        self
    }

    /// Adds multiple object proof targets by object ID.
    pub fn objects(mut self, object_ids: impl IntoIterator<Item = ObjectId>) -> Self {
        for object_id in object_ids {
            self.push_target(ProofTarget::Object(object_id));
        }
        self
    }

    /// Adds an event proof target.
    pub fn event(mut self, event_id: EventID) -> Self {
        self.push_target(ProofTarget::Event(event_id));
        self
    }

    /// Adds multiple event proof targets.
    pub fn events(mut self, event_ids: impl IntoIterator<Item = EventID>) -> Self {
        for event_id in event_ids {
            self.push_target(ProofTarget::Event(event_id));
        }
        self
    }

    /// Builds the requested proof from the configured source.
    pub async fn build(self) -> Result<Proof, ProofBuilderError> {
        if self.targets.is_empty() {
            return Err(ProofBuilderError::MissingTarget);
        }

        self.build_proof().await
    }

    async fn build_proof(&self) -> Result<Proof, ProofBuilderError> {
        let mut selected_transaction = None;
        let mut object_ids = Vec::new();
        let mut events = Vec::new();

        for target in self.targets.iter().copied() {
            match target {
                ProofTarget::Transaction(transaction_digest) => {
                    Self::ensure_same_transaction(&mut selected_transaction, target, transaction_digest)?;
                }
                ProofTarget::Object(object_id) => object_ids.push(object_id),
                ProofTarget::Event(event_id) => {
                    Self::ensure_same_transaction(&mut selected_transaction, target, event_id.tx_digest)?;
                    events.push(event_id);
                }
            }
        }

        let (transaction_digest, transaction, objects) = if let Some(transaction_digest) = selected_transaction {
            let transaction = self.fetch_transaction(transaction_digest).await?;
            let changed_objects = transaction.effects.all_changed_objects();
            let mut objects = Vec::with_capacity(object_ids.len());

            for object_id in object_ids {
                let object_ref = changed_objects
                    .iter()
                    .find_map(|(object_ref, _, _)| (object_ref.object_id == object_id).then_some(*object_ref))
                    .ok_or(ProofBuilderError::ObjectNotChangedByTransaction {
                        object_id,
                        transaction_digest,
                    })?;
                objects.push(self.fetch_object(object_id, Some(object_ref)).await?);
            }

            (transaction_digest, transaction, objects)
        } else {
            let mut objects = Vec::with_capacity(object_ids.len());

            for object_id in object_ids {
                let (object_ref, object) = self.fetch_object(object_id, None).await?;
                Self::ensure_same_transaction(
                    &mut selected_transaction,
                    ProofTarget::Object(object_id),
                    object.previous_transaction,
                )?;
                objects.push((object_ref, object));
            }

            let transaction_digest =
                selected_transaction.expect("ProofBuilder only builds a proof for non-empty targets");
            let transaction = self.fetch_transaction(transaction_digest).await?;

            (transaction_digest, transaction, objects)
        };

        let target = ProofTarget::Transaction(transaction_digest);
        let chain_identifier = self
            .source
            .chain_identifier()
            .await
            .map_err(|source| ProofBuilderError::Source { target, source })?;
        let checkpoint = self
            .source
            .checkpoint(transaction.checkpoint_sequence_number)
            .await
            .map_err(|source| ProofBuilderError::Source { target, source })?;
        let transaction_proof = TransactionProof::new(
            checkpoint.contents,
            transaction.transaction,
            transaction.effects,
            transaction.events,
        );
        let mut proof = Proof::new(
            chain_identifier,
            ProofTargets::new(),
            checkpoint.summary,
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
                .ok_or(ProofBuilderError::TargetNotFound {
                    target: ProofTarget::Event(event_id),
                })?;
            proof.target = proof.target.add_event(event_id, event);
        }

        Ok(proof)
    }

    async fn fetch_transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<crate::SourceTransaction, ProofBuilderError> {
        let target = ProofTarget::Transaction(transaction_digest);
        self.source
            .transaction(transaction_digest)
            .await
            .map_err(|source| ProofBuilderError::Source { target, source })?
            .ok_or(ProofBuilderError::TargetNotFound { target })
    }

    async fn fetch_object(
        &self,
        object_id: ObjectId,
        expected_ref: Option<ObjectReference>,
    ) -> Result<(ObjectReference, Object), ProofBuilderError> {
        let target = ProofTarget::Object(object_id);
        let object = self
            .source
            .object(object_id, expected_ref.map(|object_ref| object_ref.version))
            .await
            .map_err(|source| ProofBuilderError::Source { target, source })?
            .ok_or(ProofBuilderError::TargetNotFound { target })?;
        let object_ref = object.as_inner().object_ref();

        if object_ref.object_id != object_id || expected_ref.is_some_and(|expected| expected != object_ref) {
            return Err(ProofBuilderError::ObjectReferenceMismatch { object_id });
        }

        Ok((object_ref, object))
    }

    fn ensure_same_transaction(
        selected: &mut Option<TransactionDigest>,
        target: ProofTarget,
        transaction_digest: TransactionDigest,
    ) -> Result<(), ProofBuilderError> {
        if let Some(expected) = selected {
            if *expected != transaction_digest {
                return Err(ProofBuilderError::TargetTransactionMismatch {
                    target,
                    expected: *expected,
                    actual: transaction_digest,
                });
            }
        } else {
            *selected = Some(transaction_digest);
        }

        Ok(())
    }

    fn push_target(&mut self, target: ProofTarget) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "native-grpc")]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mainnet_uses_the_sdk_mainnet_endpoint() {
        let builder = ProofBuilder::mainnet().expect("mainnet builder must be configured");
        let expected = GrpcClient::new_mainnet().expect("SDK mainnet client must be configured");

        assert_eq!(builder.source.uri(), expected.uri());
    }

    #[tokio::test]
    async fn testnet_uses_the_sdk_testnet_endpoint() {
        let builder = ProofBuilder::testnet().expect("testnet builder must be configured");
        let expected = GrpcClient::new_testnet().expect("SDK testnet client must be configured");

        assert_eq!(builder.source.uri(), expected.uri());
    }

    #[tokio::test]
    async fn devnet_uses_the_sdk_devnet_endpoint() {
        let builder = ProofBuilder::devnet().expect("devnet builder must be configured");
        let expected = GrpcClient::new_devnet().expect("SDK devnet client must be configured");

        assert_eq!(builder.source.uri(), expected.uri());
    }
}
