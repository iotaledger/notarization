// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::{ObjectId, ObjectReference, TransactionDigest};
use iota_types::effects::TransactionEffectsExt;
use iota_types::event::EventID;
use iota_types::object::Object;

use crate::{Proof, ProofTargets, ProofV1, Source, SourceError, TransactionProof};

/// Error returned when a proof cannot be constructed by [`ProofBuilder`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofBuilderError {
    /// No proof request was selected before building.
    #[error("proof builder requires a request")]
    MissingRequest,
    /// The configured source failed while reading proof evidence.
    #[error("source failed while reading proof evidence")]
    Source {
        /// Underlying source failure.
        #[source]
        source: SourceError,
    },
    /// The source did not return a requested transaction.
    #[error("transaction {transaction_digest} was not found")]
    TransactionNotFound {
        /// Transaction digest that was not returned.
        transaction_digest: TransactionDigest,
    },
    /// The source did not return a requested object.
    #[error("object {object_id} was not found")]
    ObjectNotFound {
        /// Object ID that was not returned.
        object_id: ObjectId,
    },
    /// The requested event was not present in its transaction.
    #[error("event {event_id:?} was not found")]
    EventNotFound {
        /// Event ID that was not present.
        event_id: EventID,
    },
    /// The returned object does not match the requested ID or transaction effects.
    #[error("object {object_id} reference does not match the requested object")]
    ObjectReferenceMismatch {
        /// Requested object ID.
        object_id: ObjectId,
    },
    /// The selected transaction did not write a provable value for the requested object.
    #[error(
        "transaction {transaction_digest} did not write a provable value for object {object_id}; deleted and wrapped objects are unsupported"
    )]
    ObjectNotChangedByTransaction {
        /// Requested object ID.
        object_id: ObjectId,
        /// Transaction selected by the other proof requests.
        transaction_digest: TransactionDigest,
    },
    /// The requests belong to different transactions.
    #[error("proof requests belong to different transactions: {actual}, expected {expected}")]
    TransactionMismatch {
        /// Transaction selected by the first request.
        expected: TransactionDigest,
        /// Transaction selected by a conflicting request.
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
    transaction_digests: Vec<TransactionDigest>,
    object_ids: Vec<ObjectId>,
    event_ids: Vec<EventID>,
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
            transaction_digests: Vec::new(),
            object_ids: Vec::new(),
            event_ids: Vec::new(),
        }
    }

    /// Adds a transaction proof request.
    pub fn transaction(mut self, transaction_digest: TransactionDigest) -> Self {
        Self::push_unique(&mut self.transaction_digests, transaction_digest);
        self
    }

    /// Adds an object proof request by object ID.
    ///
    /// Without a transaction or event request, the source resolves the object's
    /// latest version at proof construction time.
    pub fn object(mut self, object_id: ObjectId) -> Self {
        Self::push_unique(&mut self.object_ids, object_id);
        self
    }

    /// Adds multiple object proof requests by object ID.
    ///
    /// Object resolution follows the build-time semantics of [`Self::object`].
    pub fn objects(mut self, object_ids: impl IntoIterator<Item = ObjectId>) -> Self {
        for object_id in object_ids {
            Self::push_unique(&mut self.object_ids, object_id);
        }
        self
    }

    /// Adds an event proof request.
    pub fn event(mut self, event_id: EventID) -> Self {
        Self::push_unique(&mut self.event_ids, event_id);
        self
    }

    /// Adds multiple event proof requests.
    pub fn events(mut self, event_ids: impl IntoIterator<Item = EventID>) -> Self {
        for event_id in event_ids {
            Self::push_unique(&mut self.event_ids, event_id);
        }
        self
    }

    /// Builds the requested proof from the configured source.
    pub async fn build(self) -> Result<Proof, ProofBuilderError> {
        if self.transaction_digests.is_empty() && self.object_ids.is_empty() && self.event_ids.is_empty() {
            return Err(ProofBuilderError::MissingRequest);
        }

        self.build_proof().await
    }

    async fn build_proof(&self) -> Result<Proof, ProofBuilderError> {
        let mut selected_transaction = None;

        for transaction_digest in self.transaction_digests.iter().copied() {
            Self::ensure_same_transaction(&mut selected_transaction, transaction_digest)?;
        }
        for event_id in &self.event_ids {
            Self::ensure_same_transaction(&mut selected_transaction, event_id.tx_digest)?;
        }

        let (transaction, objects) = if let Some(transaction_digest) = selected_transaction {
            let transaction = self.fetch_transaction(transaction_digest).await?;
            let changed_objects = transaction.effects.all_changed_objects();
            let mut objects = Vec::with_capacity(self.object_ids.len());

            for object_id in self.object_ids.iter().copied() {
                let object_ref = changed_objects
                    .iter()
                    .find_map(|(object_ref, _, _)| (object_ref.object_id == object_id).then_some(*object_ref))
                    .ok_or(ProofBuilderError::ObjectNotChangedByTransaction {
                        object_id,
                        transaction_digest,
                    })?;
                objects.push(self.fetch_object(object_id, Some(object_ref)).await?);
            }

            (transaction, objects)
        } else {
            let mut objects = Vec::with_capacity(self.object_ids.len());

            for object_id in self.object_ids.iter().copied() {
                let object = self.fetch_object(object_id, None).await?;
                Self::ensure_same_transaction(&mut selected_transaction, object.previous_transaction)?;
                objects.push(object);
            }

            let transaction_digest =
                selected_transaction.expect("ProofBuilder only builds a proof for non-empty requests");
            let transaction = self.fetch_transaction(transaction_digest).await?;

            (transaction, objects)
        };

        let chain_identifier = self
            .source
            .chain_identifier()
            .await
            .map_err(|source| ProofBuilderError::Source { source })?;
        let checkpoint = self
            .source
            .checkpoint(transaction.checkpoint_sequence_number)
            .await
            .map_err(|source| ProofBuilderError::Source { source })?;
        let transaction_events = if self.event_ids.is_empty() {
            None
        } else {
            let events = transaction.events.ok_or_else(|| ProofBuilderError::EventNotFound {
                event_id: self.event_ids[0],
            })?;

            for event_id in &self.event_ids {
                let event_exists = usize::try_from(event_id.event_seq)
                    .ok()
                    .is_some_and(|index| events.get(index).is_some());
                if !event_exists {
                    return Err(ProofBuilderError::EventNotFound { event_id: *event_id });
                }
            }

            Some(events)
        };
        let transaction_proof = TransactionProof::new(transaction.transaction, transaction.effects, transaction_events);
        let mut targets = ProofTargets::new();
        if let Some(transaction_digest) = self.transaction_digests.first().copied() {
            targets = targets.set_transaction(transaction_digest);
        }
        for object in objects {
            targets = targets.add_object(object);
        }
        for event_id in self.event_ids.iter().copied() {
            targets = targets.add_event(event_id);
        }

        Ok(ProofV1::new(
            chain_identifier,
            targets,
            checkpoint.summary,
            checkpoint.contents,
            transaction_proof,
        )
        .into())
    }

    async fn fetch_transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<crate::SourceTransaction, ProofBuilderError> {
        self.source
            .transaction(transaction_digest)
            .await
            .map_err(|source| ProofBuilderError::Source { source })?
            .ok_or(ProofBuilderError::TransactionNotFound { transaction_digest })
    }

    async fn fetch_object(
        &self,
        object_id: ObjectId,
        expected_ref: Option<ObjectReference>,
    ) -> Result<Object, ProofBuilderError> {
        let object = self
            .source
            .object(object_id, expected_ref.map(|object_ref| object_ref.version))
            .await
            .map_err(|source| ProofBuilderError::Source { source })?
            .ok_or(ProofBuilderError::ObjectNotFound { object_id })?;
        let object_ref = object.as_inner().object_ref();

        if object_ref.object_id != object_id || expected_ref.is_some_and(|expected| expected != object_ref) {
            return Err(ProofBuilderError::ObjectReferenceMismatch { object_id });
        }

        Ok(object)
    }

    fn ensure_same_transaction(
        selected: &mut Option<TransactionDigest>,
        transaction_digest: TransactionDigest,
    ) -> Result<(), ProofBuilderError> {
        if let Some(expected) = selected {
            if *expected != transaction_digest {
                return Err(ProofBuilderError::TransactionMismatch {
                    expected: *expected,
                    actual: transaction_digest,
                });
            }
        } else {
            *selected = Some(transaction_digest);
        }

        Ok(())
    }

    fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
        if !values.contains(&value) {
            values.push(value);
        }
    }
}
