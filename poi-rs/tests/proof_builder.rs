// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use iota_sdk_types::{ObjectId, TransactionDigest, Version};
use iota_types::base_types::dbg_object_id;
use iota_types::{
    committee::{Committee, EpochId},
    digests::ChainIdentifier,
    event::EventID,
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
};
use poi_rs::{
    ProofBuilder, ProofBuilderError, Source, SourceCheckpoint, SourceError, SourceErrorKind, SourceTarget,
    SourceTransaction,
};
use utils::{genesis_chain_identifier, grpc_client, object_transfer_tx, staking_tx, start_test_cluster, transfer_tx};

struct RejectingSource;

#[async_trait]
impl Source for RejectingSource {
    type CommitteeError = std::convert::Infallible;

    async fn chain_identifier(&self, _transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        unreachable!("rejected transactions do not resolve a chain identifier")
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        Err(SourceError::transaction(
            transaction_digest,
            SourceErrorKind::TransactionNotFound,
        ))
    }

    async fn object(&self, object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
        Err(SourceError::object(object_id, SourceErrorKind::ObjectNotFound))
    }

    async fn checkpoint(
        &self,
        _transaction_digest: TransactionDigest,
        _sequence_number: u64,
    ) -> Result<SourceCheckpoint, SourceError> {
        unreachable!("rejected transactions do not resolve a checkpoint")
    }

    async fn committee(&self, _epoch: EpochId) -> Result<Committee, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve committees")
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve the current epoch")
    }

    async fn epoch_close_summary(
        &self,
        _epoch: EpochId,
    ) -> Result<Option<CertifiedCheckpointSummary>, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve epoch-close summaries")
    }
}

struct RecordingSource {
    requests: Arc<AtomicUsize>,
    transactions: Arc<Mutex<Vec<TransactionDigest>>>,
}

#[async_trait]
impl Source for RecordingSource {
    type CommitteeError = std::convert::Infallible;

    async fn chain_identifier(&self, _transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        unreachable!("rejected transactions do not resolve a chain identifier")
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.transactions
            .lock()
            .expect("recorded transactions lock must not be poisoned")
            .push(transaction_digest);

        Err(SourceError::transaction(
            transaction_digest,
            SourceErrorKind::TransactionNotFound,
        ))
    }

    async fn object(&self, object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
        Err(SourceError::object(object_id, SourceErrorKind::ObjectNotFound))
    }

    async fn checkpoint(
        &self,
        _transaction_digest: TransactionDigest,
        _sequence_number: u64,
    ) -> Result<SourceCheckpoint, SourceError> {
        unreachable!("rejected transactions do not resolve a checkpoint")
    }

    async fn committee(&self, _epoch: EpochId) -> Result<Committee, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve committees")
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve the current epoch")
    }

    async fn epoch_close_summary(
        &self,
        _epoch: EpochId,
    ) -> Result<Option<CertifiedCheckpointSummary>, Self::CommitteeError> {
        unreachable!("proof-only test source does not resolve epoch-close summaries")
    }
}

#[tokio::test]
async fn builder_accepts_a_custom_source() {
    let transaction_digest = TransactionDigest::random();

    let error = ProofBuilder::new(RejectingSource)
        .transaction(transaction_digest)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("custom source error must be preserved");
    };
    assert_eq!(source.target, SourceTarget::Transaction(transaction_digest));
    assert!(matches!(source.kind, SourceErrorKind::TransactionNotFound));
}

#[tokio::test]
async fn builder_without_a_target_is_rejected() {
    let error = ProofBuilder::new(RejectingSource).build().await.unwrap_err();

    assert!(matches!(error, ProofBuilderError::MissingTarget));
}

#[tokio::test]
async fn stacked_targets_reuse_one_transaction_request() {
    let transaction_digest = TransactionDigest::random();
    let object_a = dbg_object_id(1);
    let object_b = dbg_object_id(2);
    let event_a = EventID {
        tx_digest: transaction_digest,
        event_seq: 0,
    };
    let event_b = EventID {
        tx_digest: transaction_digest,
        event_seq: 1,
    };
    let requests = Arc::new(AtomicUsize::new(0));
    let transactions = Arc::new(Mutex::new(Vec::new()));

    let _ = ProofBuilder::new(RecordingSource {
        requests: requests.clone(),
        transactions: transactions.clone(),
    })
    .transaction(transaction_digest)
    .objects([object_a, object_b, object_a])
    .object(object_b)
    .events([event_a, event_b, event_a])
    .event(event_b)
    .build()
    .await
    .unwrap_err();

    assert_eq!(requests.load(Ordering::Relaxed), 1);
    assert_eq!(
        *transactions
            .lock()
            .expect("recorded transactions lock must not be poisoned"),
        vec![transaction_digest]
    );
}

#[tokio::test]
async fn unknown_transaction_returns_a_fetch_error() {
    let cluster = start_test_cluster().await;
    let transaction_digest = TransactionDigest::random();

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .transaction(transaction_digest)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("missing transaction must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Transaction(transaction_digest));
    assert!(matches!(source.kind, SourceErrorKind::FetchTransaction { .. }));
}

#[tokio::test]
async fn proof_uses_the_genesis_checkpoint_as_its_chain_identifier() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;

    let proof = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .transaction(transfer.digest)
        .build()
        .await
        .expect("transaction proof must be constructed");

    assert_eq!(proof.chain, genesis_chain_identifier(&cluster));
}

#[tokio::test]
async fn unknown_object_returns_a_fetch_error() {
    let cluster = start_test_cluster().await;
    let object_id = Object::immutable_for_testing().id();

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .object(object_id)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("missing object must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Object(object_id));
    assert!(matches!(source.kind, SourceErrorKind::FetchObject { .. }));
}

#[tokio::test]
async fn event_sequence_outside_the_transaction_is_rejected() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: u64::MAX,
    };

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .event(event_id)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("missing event must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Event(event_id));
    assert!(matches!(source.kind, SourceErrorKind::EventNotFound));
}

#[tokio::test]
async fn object_outside_the_event_transaction_is_rejected() {
    let cluster = start_test_cluster().await;
    let transfer = object_transfer_tx(&cluster).await;
    let staking = staking_tx(&cluster).await;
    let object_id = transfer.objects[1].object_id;
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .object(object_id)
        .event(event_id)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("mixed transactions must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Object(object_id));
    assert!(matches!(
        source.kind,
        SourceErrorKind::ObjectNotChangedByTransaction { transaction_digest }
            if transaction_digest == staking.digest
    ));
}

#[tokio::test]
async fn object_targets_from_different_transactions_are_rejected() {
    let cluster = start_test_cluster().await;
    let first = object_transfer_tx(&cluster).await;
    let second = object_transfer_tx(&cluster).await;
    let first_object_id = first.objects[1].object_id;
    let second_object_id = second.objects[1].object_id;

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .objects([first_object_id, second_object_id])
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("mixed transactions must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Object(second_object_id));
    assert!(matches!(
        source.kind,
        SourceErrorKind::TargetTransactionMismatch { expected, actual }
            if expected == first.digest && actual == second.digest
    ));
}
