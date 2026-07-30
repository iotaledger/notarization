// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use std::fs::File;

use iota_config::IOTA_GENESIS_FILENAME;
use iota_types::event::EventID;
use poi_rs::PoiClient;
use utils::{advance_to_epoch, grpc_client, object_transfer_tx, staking_tx, start_test_cluster, transfer_tx};

#[tokio::test]
async fn anchored_verification_walks_from_genesis_and_verifies_the_proof() {
    let cluster = start_test_cluster().await;
    let genesis = File::open(cluster.swarm.dir().join(IOTA_GENESIS_FILENAME))
        .expect("test cluster genesis blob must be available");
    advance_to_epoch(&cluster, 1).await;
    let transfer = transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));
    let proof = client
        .proof()
        .transaction(transfer.digest)
        .build()
        .await
        .expect("transaction proof must be constructed");

    client
        .anchored_at_genesis(genesis)
        .expect("test cluster genesis blob must load")
        .verify(&proof)
        .await
        .expect("anchored verification must authenticate the committee and verify the proof");
}

#[tokio::test]
async fn transaction_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));

    let proof = client
        .proof()
        .transaction(transfer.digest)
        .build()
        .await
        .expect("transaction proof must be constructed");

    client
        .trusted_node()
        .verify(&proof)
        .await
        .expect("transaction proof must verify");
}

#[tokio::test]
async fn object_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));

    let proof = client
        .proof()
        .object(transfer.gas_object.object_id)
        .build()
        .await
        .expect("object proof must be constructed");

    assert_eq!(proof.target.objects[0].0, transfer.gas_object);
    client
        .trusted_node()
        .verify(&proof)
        .await
        .expect("object proof must verify");
}

#[tokio::test]
async fn event_proof_verifies_with_the_resolved_committee() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };

    let proof = client
        .proof()
        .event(event_id)
        .build()
        .await
        .expect("event proof must be constructed");

    client
        .trusted_node()
        .verify(&proof)
        .await
        .expect("event proof must verify");
}

#[tokio::test]
async fn multiple_object_targets_share_one_verified_transaction_proof() {
    let cluster = start_test_cluster().await;
    let transfer = object_transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));

    let proof = client
        .proof()
        .objects(transfer.objects.map(|object_ref| object_ref.object_id))
        .build()
        .await
        .expect("stacked object proof must be constructed");

    assert_eq!(proof.transaction_proof.transaction.digest(), &transfer.digest);
    assert_eq!(proof.target.objects.len(), 2);
    client
        .trusted_node()
        .verify(&proof)
        .await
        .expect("stacked object proof must verify");
}

#[tokio::test]
async fn object_and_event_targets_share_one_verified_transaction_proof() {
    let cluster = start_test_cluster().await;
    let staking = staking_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));
    let event_id = EventID {
        tx_digest: staking.digest,
        event_seq: 0,
    };

    let proof = client
        .proof()
        .object(staking.gas_object.object_id)
        .event(event_id)
        .build()
        .await
        .expect("mixed target proof must be constructed");

    assert_eq!(proof.transaction_proof.transaction.digest(), &staking.digest);
    assert_eq!(proof.target.objects[0].0, staking.gas_object);
    assert_eq!(proof.target.objects.len(), 1);
    assert_eq!(proof.target.events.len(), 1);
    client
        .trusted_node()
        .verify(&proof)
        .await
        .expect("mixed target proof must verify");
}
