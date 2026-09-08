// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use std::sync::{Arc, Mutex};

use iota_sdk_types::TransactionDigest;
use iota_types::event::EventID;
use iota_types::object::Object;
use poi_rs::{PoiClient, ProofBuilderError, ProofVerifier, Source, SourceError};
use utils::sources::{MissingSource, RecordingSource, RejectingSource};
use utils::{genesis_chain_identifier, grpc_client, object_transfer_tx, staking_tx, start_test_cluster, transfer_tx};

#[tokio::test]
async fn client_uses_a_custom_source_for_proof_building() {
    let transaction_digest = TransactionDigest::random();

    let error = PoiClient::new(RejectingSource)
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .expect_err("the custom source error must be returned");

    let ProofBuilderError::Source { source } = error else {
        panic!("custom source error must be preserved");
    };
    assert!(matches!(source, SourceError::Request { .. }));
}

#[tokio::test]
async fn proof_requires_at_least_one_target() {
    let error = PoiClient::new(RejectingSource)
        .proof()
        .build()
        .await
        .expect_err("a proof without a target must be rejected");

    assert!(matches!(error, ProofBuilderError::MissingTarget));
}

#[tokio::test]
async fn stacked_targets_are_deduplicated_and_reuse_transaction_evidence() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let object_id = staking.gas_object.object_id;
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };
    let transactions = Arc::new(Mutex::new(Vec::new()));
    let source = RecordingSource::new(grpc_client(&cluster), transactions.clone());

    let proof = PoiClient::new(source)
        .proof()
        .transaction(staking.digest)
        .objects([object_id, object_id])
        .object(object_id)
        .events([event_id, event_id])
        .event(event_id)
        .build()
        .await
        .expect("stacked targets from one transaction must produce a proof");

    assert_eq!(
        *transactions
            .lock()
            .expect("recorded transactions lock must not be poisoned"),
        vec![staking.digest]
    );
    assert_eq!(proof.targets().transaction, Some(staking.digest));
    assert_eq!(proof.targets().objects.len(), 1);
    assert_eq!(proof.targets().events.len(), 1);
    let _verified = ProofVerifier::new(&cluster.committee())
        .verify(&proof)
        .expect("the stacked-target proof must verify offline");
}

#[tokio::test]
async fn transaction_not_returned_by_the_source_is_reported_as_missing() {
    let transaction_digest = TransactionDigest::random();

    let error = PoiClient::new(MissingSource)
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .expect_err("a transaction omitted by the source must be rejected");

    let ProofBuilderError::TransactionNotFound {
        transaction_digest: missing_transaction,
    } = error
    else {
        panic!("an omitted transaction must return a transaction-not-found error");
    };
    assert_eq!(missing_transaction, transaction_digest);
}

#[tokio::test]
async fn checkpoint_not_returned_by_the_source_is_reported_as_missing() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = grpc_client(&cluster);
    let checkpoint_sequence_number = client
        .transaction(transfer.digest)
        .await
        .expect("transaction request must succeed")
        .expect("executed transaction must exist")
        .checkpoint_sequence_number;
    let source = RecordingSource::new(client, Arc::new(Mutex::new(Vec::new()))).without_checkpoints();

    let error = PoiClient::new(source)
        .proof()
        .transaction(transfer.digest)
        .build()
        .await
        .expect_err("a checkpoint omitted by the source must be rejected");

    assert!(matches!(
        error,
        ProofBuilderError::CheckpointNotFound { sequence_number }
            if sequence_number == checkpoint_sequence_number
    ));
}

#[tokio::test]
async fn proof_uses_the_genesis_checkpoint_as_its_chain_identifier() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;

    let proof = PoiClient::from_grpc_client(grpc_client(&cluster))
        .proof()
        .transaction(transfer.digest)
        .build()
        .await
        .expect("transaction proof must be constructed");

    assert_eq!(proof.chain(), &genesis_chain_identifier(&cluster));
}

#[tokio::test]
async fn object_not_returned_by_the_source_is_reported_as_missing() {
    let object_id = Object::immutable_for_testing().id();

    let error = PoiClient::new(MissingSource)
        .proof()
        .object(object_id)
        .build()
        .await
        .expect_err("an object omitted by the source must be rejected");

    let ProofBuilderError::ObjectNotFound {
        object_id: missing_object,
    } = error
    else {
        panic!("an omitted object must return an object-not-found error");
    };
    assert_eq!(missing_object, object_id);
}

#[tokio::test]
async fn object_that_does_not_match_the_requested_reference_is_rejected() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let object_id = transfer.gas_object.object_id;
    let transactions = Arc::new(Mutex::new(Vec::new()));
    let source =
        RecordingSource::new(grpc_client(&cluster), transactions).with_object_override(Object::immutable_for_testing());

    let error = PoiClient::new(source)
        .proof()
        .transaction(transfer.digest)
        .object(object_id)
        .build()
        .await
        .expect_err("an object that does not match the effects reference must be rejected");

    assert!(matches!(
        error,
        ProofBuilderError::ObjectReferenceMismatch {
            object_id: returned_object_id
        } if returned_object_id == object_id
    ));
}

#[tokio::test]
async fn explicit_transaction_and_event_from_different_transactions_are_rejected_without_fetching() {
    let transaction_digest = TransactionDigest::new([1; 32]);
    let event_id = EventID {
        tx_digest: TransactionDigest::new([2; 32]),
        event_seq: 0,
    };

    let error = PoiClient::new(MissingSource)
        .proof()
        .transaction(transaction_digest)
        .event(event_id)
        .build()
        .await
        .expect_err("targets from different transactions must be rejected");

    assert!(matches!(
        error,
        ProofBuilderError::TransactionMismatch { expected, actual }
            if expected == transaction_digest && actual == event_id.tx_digest
    ));
}

#[tokio::test]
async fn event_sequence_outside_the_transaction_is_rejected() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: u64::MAX,
    };

    let error = PoiClient::from_grpc_client(grpc_client(&cluster))
        .proof()
        .event(event_id)
        .build()
        .await
        .expect_err("an event sequence outside the transaction must be rejected");

    let ProofBuilderError::EventNotFound {
        event_id: missing_event,
    } = error
    else {
        panic!("missing event must return an event-not-found error");
    };
    assert_eq!(missing_event, event_id);
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

    let error = PoiClient::from_grpc_client(grpc_client(&cluster))
        .proof()
        .object(object_id)
        .event(event_id)
        .build()
        .await
        .expect_err("an object outside the event transaction must be rejected");

    let ProofBuilderError::ObjectNotChangedByTransaction {
        object_id: returned_object_id,
        transaction_digest,
    } = error
    else {
        panic!("unrelated object must return a proof-builder error");
    };
    assert_eq!(returned_object_id, object_id);
    assert_eq!(transaction_digest, staking.digest);
}

#[tokio::test]
async fn object_targets_from_different_transactions_are_rejected() {
    let cluster = start_test_cluster().await;
    let first = object_transfer_tx(&cluster).await;
    let second = object_transfer_tx(&cluster).await;
    let first_object_id = first.objects[1].object_id;
    let second_object_id = second.objects[1].object_id;

    let error = PoiClient::from_grpc_client(grpc_client(&cluster))
        .proof()
        .objects([first_object_id, second_object_id])
        .build()
        .await
        .expect_err("objects from different transactions must be rejected");

    let ProofBuilderError::TransactionMismatch { expected, actual } = error else {
        panic!("mixed transactions must return a proof-builder error");
    };
    assert_eq!(expected, first.digest);
    assert_eq!(actual, second.digest);
}
