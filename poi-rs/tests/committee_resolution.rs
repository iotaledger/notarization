// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use std::fs::File;

use iota_config::IOTA_GENESIS_FILENAME;
use iota_grpc_client::Client as GrpcClient;
use poi_rs::{
    CommitteeCache, CommitteeCacheKey, CommitteeResolution, CommitteeResolutionErrorKind, MemoryCommitteeCache,
    PoiClient,
};
use utils::{advance_to_epoch, genesis_chain_identifier, grpc_client, start_test_cluster};

use crate::utils::committee_at;

fn disconnected_client() -> GrpcClient {
    GrpcClient::new("http://127.0.0.1:1").expect("disconnected gRPC client must be constructed")
}

#[tokio::test]
async fn genesis_anchored_client_authenticates_committee_across_epochs() {
    let cluster = start_test_cluster().await;
    let expected = advance_to_epoch(&cluster, 10).await;
    let genesis_path = cluster.swarm.dir().join(IOTA_GENESIS_FILENAME);
    let genesis = File::open(genesis_path).expect("test cluster genesis blob must be available");
    let cache = MemoryCommitteeCache::new();

    let resolution = CommitteeResolution::from_genesis_with_cache(genesis, cache.clone())
        .expect("test cluster genesis committee must be extractable");
    let resolver = PoiClient::new(grpc_client(&cluster)).verifier(resolution);

    let resolved = resolver
        .resolve(10)
        .await
        .expect("epoch 10 committee must resolve from genesis");

    assert_eq!(resolved, expected[10]);
    assert_eq!(
        cache
            .committee(CommitteeCacheKey::new(genesis_chain_identifier(&cluster), 10))
            .await
            .expect("caller-provided cache must remain readable"),
        Some(expected[10].clone())
    );
}

#[tokio::test]
async fn committee_anchored_client_returns_its_trust_anchor_without_fetching() {
    let trusted_committee = committee_at(7);
    let resolver =
        PoiClient::new(disconnected_client()).verifier(CommitteeResolution::anchored(trusted_committee.clone()));

    let resolved = resolver
        .resolve(7)
        .await
        .expect("the trusted committee must resolve without fetching");

    assert_eq!(resolved, trusted_committee);
}

#[tokio::test]
async fn committee_anchored_client_rejects_epochs_before_its_anchor_without_fetching() {
    let resolver = PoiClient::new(disconnected_client()).verifier(CommitteeResolution::anchored(committee_at(7)));

    let error = resolver
        .resolve(6)
        .await
        .expect_err("an anchored resolver cannot walk backwards");

    assert_eq!(error.target_epoch, 6);
    assert!(matches!(
        error.kind,
        CommitteeResolutionErrorKind::TargetBeforeAnchor { anchor_epoch: 7 }
    ));
}

#[tokio::test]
async fn genesis_anchored_client_rejects_epochs_ahead_of_the_node() {
    let cluster = start_test_cluster().await;
    let genesis_path = cluster.swarm.dir().join(IOTA_GENESIS_FILENAME);
    let genesis = File::open(genesis_path).expect("test cluster genesis blob must be available");
    let resolution =
        CommitteeResolution::from_genesis(genesis).expect("test cluster genesis committee must be extractable");
    let resolver = PoiClient::new(grpc_client(&cluster)).verifier(resolution);

    let error = resolver
        .resolve(1)
        .await
        .expect_err("an epoch beyond the node's current epoch must be rejected");

    assert!(matches!(
        error.kind,
        CommitteeResolutionErrorKind::TargetAheadOfNode { current_epoch: 0 }
    ));
}

#[tokio::test]
async fn trusted_node_client_returns_the_committee_reported_by_its_source() {
    let cluster = start_test_cluster().await;
    let resolver = PoiClient::new(grpc_client(&cluster)).verifier(CommitteeResolution::TrustedNode);

    let resolved = resolver
        .resolve(0)
        .await
        .expect("trusted node must return its genesis committee");

    assert_eq!(resolved, *cluster.committee());
}
