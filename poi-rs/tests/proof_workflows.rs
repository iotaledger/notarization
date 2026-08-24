// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use std::fs::File;

use iota_config::IOTA_GENESIS_FILENAME;
use iota_types::event::EventID;
use poi_rs::{CommitteeResolution, PoiClient};
use utils::{advance_to_epoch, grpc_client, object_transfer_tx, staking_tx, start_test_cluster, transfer_tx};

#[tokio::test]
async fn client_builds_and_verifies_a_transaction_proof_from_genesis() {
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

    assert_eq!(proof.targets().transaction, Some(transfer.digest));
    assert!(proof.targets().objects.is_empty());
    assert!(proof.targets().events.is_empty());
    assert!(proof.transaction_proof().events.is_none());

    let resolution = CommitteeResolution::from_genesis(genesis).expect("test cluster genesis blob must load");
    client
        .verifier(resolution)
        .verify(&proof)
        .await
        .expect("anchored verification must authenticate the committee and verify the proof");
}

#[tokio::test]
async fn client_builds_and_verifies_an_object_proof_with_a_trusted_node() {
    let cluster = start_test_cluster().await;
    let transfer = transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));

    let proof = client
        .proof()
        .object(transfer.gas_object.object_id)
        .build()
        .await
        .expect("object proof must be constructed");

    assert!(proof.targets().transaction.is_none());
    assert_eq!(proof.targets().objects[0].as_inner().object_ref(), transfer.gas_object);
    assert!(proof.targets().events.is_empty());
    assert!(proof.transaction_proof().events.is_none());
    client
        .verifier(CommitteeResolution::TrustedNode)
        .verify(&proof)
        .await
        .expect("object proof must verify");
}

#[tokio::test]
async fn client_builds_and_verifies_an_event_proof_with_a_trusted_node() {
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

    assert!(proof.targets().transaction.is_none());
    assert!(proof.targets().objects.is_empty());
    assert_eq!(proof.targets().events, vec![event_id]);
    assert!(proof.transaction_proof().events.is_some());

    client
        .verifier(CommitteeResolution::TrustedNode)
        .verify(&proof)
        .await
        .expect("event proof must verify");
}

#[tokio::test]
async fn client_builds_one_verified_proof_for_multiple_objects() {
    let cluster = start_test_cluster().await;
    let transfer = object_transfer_tx(&cluster).await;
    let client = PoiClient::from_grpc_client(grpc_client(&cluster));

    let proof = client
        .proof()
        .objects(transfer.objects.map(|object_ref| object_ref.object_id))
        .build()
        .await
        .expect("stacked object proof must be constructed");

    assert_eq!(proof.transaction_proof().transaction.digest(), &transfer.digest);
    assert_eq!(proof.targets().objects.len(), 2);
    client
        .verifier(CommitteeResolution::TrustedNode)
        .verify(&proof)
        .await
        .expect("stacked object proof must verify");
}

#[tokio::test]
async fn client_builds_one_verified_proof_for_object_and_event_targets() {
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

    assert_eq!(proof.transaction_proof().transaction.digest(), &staking.digest);
    assert_eq!(proof.targets().objects[0].as_inner().object_ref(), staking.gas_object);
    assert_eq!(proof.targets().objects.len(), 1);
    assert_eq!(proof.targets().events.len(), 1);
    client
        .verifier(CommitteeResolution::TrustedNode)
        .verify(&proof)
        .await
        .expect("mixed target proof must verify");
}
