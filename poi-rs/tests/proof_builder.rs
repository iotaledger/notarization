// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use async_trait::async_trait;
use iota_types::base_types::ObjectRef;
use iota_types::{digests::TransactionDigest, event::EventID, object::Object};
use poi_rs::{Proof, ProofBuilder, ProofBuilderError, Source, SourceError, SourceErrorKind, SourceTarget};
use utils::{grpc_client, staking_tx, start_test_cluster};

struct RejectingSource;

#[async_trait]
impl Source for RejectingSource {
    async fn transaction(&self, transaction_digest: TransactionDigest) -> Result<Proof, SourceError> {
        Err(SourceError::transaction(
            transaction_digest,
            SourceErrorKind::TransactionNotFound,
        ))
    }

    async fn object(&self, object_ref: ObjectRef) -> Result<Proof, SourceError> {
        Err(SourceError::object(object_ref, SourceErrorKind::ObjectNotFound))
    }

    async fn event(&self, event_id: EventID) -> Result<Proof, SourceError> {
        Err(SourceError::event(event_id, SourceErrorKind::EventNotFound))
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
async fn multiple_targets_are_rejected_until_stacking_is_supported() {
    let error = ProofBuilder::new(RejectingSource)
        .transaction(TransactionDigest::random())
        .transaction(TransactionDigest::random())
        .build()
        .await
        .unwrap_err();

    assert!(matches!(error, ProofBuilderError::MultipleTargets));
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
async fn unknown_object_returns_a_fetch_error() {
    let cluster = start_test_cluster().await;
    let object_ref = Object::immutable_for_testing().as_inner().object_ref();

    let error = ProofBuilder::from_grpc_client(grpc_client(&cluster))
        .object(object_ref)
        .build()
        .await
        .unwrap_err();

    let ProofBuilderError::Source { source } = error else {
        panic!("missing object must return a source error");
    };
    assert_eq!(source.target, SourceTarget::Object(object_ref));
    assert!(matches!(source.kind, SourceErrorKind::FetchObject { .. }));
}

#[tokio::test]
async fn event_sequence_outside_the_transaction_is_rejected() {
    let cluster = start_test_cluster().await;
    let transaction_digest = staking_tx(&cluster).await;
    let event_id = EventID {
        tx_digest: transaction_digest,
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
