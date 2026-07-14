// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_types::event::EventID;
use poi_rs::{CommitteeResolver, ProofBuilder, ProofVerifier};
use utils::{grpc_client, staking_tx, start_test_cluster, transfer_tx};

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
        .object(transfer.gas_object)
        .build()
        .await
        .expect("object proof must be constructed");
    let committee = CommitteeResolver::node(client)
        .resolve(proof.checkpoint_summary.epoch())
        .await
        .expect("checkpoint committee must resolve");

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("object proof must verify");
}

#[tokio::test]
async fn event_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let transaction_digest = staking_tx(&cluster).await;
    let client = grpc_client(&cluster);
    let event_id = EventID {
        tx_digest: transaction_digest,
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
