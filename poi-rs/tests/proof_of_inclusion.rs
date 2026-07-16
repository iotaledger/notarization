// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_types::event::EventID;
use poi_rs::{CommitteeResolver, ProofBuilder, ProofVerifier};
use utils::{grpc_client, object_transfer_tx, staking_tx, start_test_cluster, transfer_tx};

#[tokio::test]
async fn transaction_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = grpc_client(&cluster);

    let proof = ProofBuilder::from_grpc_client(client.clone())
        .transaction(transfer.digest)
        .build()
        .await
        .expect("transaction proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("transaction proof must verify");
}

#[tokio::test]
async fn object_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = grpc_client(&cluster);

    let proof = ProofBuilder::from_grpc_client(client.clone())
        .object(transfer.gas_object.object_id)
        .build()
        .await
        .expect("object proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    assert_eq!(proof.target.objects[0].0, transfer.gas_object);
    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("object proof must verify");
}

#[tokio::test]
async fn event_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let client = grpc_client(&cluster);
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };

    let proof = ProofBuilder::from_grpc_client(client.clone())
        .event(event_id)
        .build()
        .await
        .expect("event proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("event proof must verify");
}

#[tokio::test]
async fn multiple_object_targets_share_one_verified_transaction_proof() {
    let cluster = start_test_cluster().await;
    let transfer = object_transfer_tx(&cluster).await;
    let client = grpc_client(&cluster);

    let proof = ProofBuilder::from_grpc_client(client.clone())
        .objects(transfer.objects.map(|object_ref| object_ref.object_id))
        .build()
        .await
        .expect("stacked object proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    assert_eq!(proof.transaction_proof.transaction.digest(), &transfer.digest);
    assert_eq!(proof.target.objects.len(), 2);
    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("stacked object proof must verify");
}

#[tokio::test]
async fn object_and_event_targets_share_one_verified_transaction_proof() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let client = grpc_client(&cluster);
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };

    let proof = ProofBuilder::from_grpc_client(client.clone())
        .object(staking.gas_object.object_id)
        .event(event_id)
        .build()
        .await
        .expect("mixed target proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    assert_eq!(proof.transaction_proof.transaction.digest(), &staking.digest);
    assert_eq!(proof.target.objects[0].0, staking.gas_object);
    assert_eq!(proof.target.objects.len(), 1);
    assert_eq!(proof.target.events.len(), 1);
    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("mixed target proof must verify");
}
