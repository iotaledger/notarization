// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_grpc_client::Client as GrpcClient;
use iota_types::committee::Committee;
use poi_rs::{CommitteeCache, CommitteeResolutionErrorKind, CommitteeResolver, MemoryCommitteeCache};
use utils::{advance_to_epoch, genesis_committee, grpc_client, start_test_cluster};

fn committee_at(epoch: u64) -> Committee {
    let (committee, _) = Committee::new_simple_test_committee();
    Committee::new(epoch, committee.voting_rights.iter().cloned().collect())
}

fn disconnected_client() -> GrpcClient {
    GrpcClient::new("http://127.0.0.1:1").expect("disconnected gRPC client must be constructed")
}

#[tokio::test]
async fn genesis_anchor_authenticates_committees_through_epoch_ten() {
    let cluster = start_test_cluster().await;
    let genesis = genesis_committee(&cluster);
    let expected = advance_to_epoch(&cluster, 10).await;
    let cache = MemoryCommitteeCache::new();
    let resolver = CommitteeResolver::anchor_with_cache(grpc_client(&cluster), genesis, cache.clone());

    let resolved = resolver
        .resolve(10)
        .await
        .expect("epoch 10 committee must resolve from genesis");

    assert_eq!(resolved, expected[10]);
    assert_eq!(cache.len().await, 10);
    for epoch in 1..=10 {
        assert_eq!(
            cache.committee(epoch).await.unwrap(),
            Some(expected[epoch as usize].clone())
        );
    }
}

#[tokio::test]
async fn epoch_before_the_trust_anchor_is_rejected() {
    let resolver = CommitteeResolver::anchor(disconnected_client(), committee_at(7));

    let error = resolver.resolve(6).await.unwrap_err();

    assert_eq!(error.target_epoch, 6);
    assert!(matches!(
        error.kind,
        CommitteeResolutionErrorKind::TargetBeforeAnchor { anchor_epoch: 7 }
    ));
}

#[tokio::test]
async fn epoch_ahead_of_the_node_is_rejected_without_caching() {
    let cluster = start_test_cluster().await;
    let cache = MemoryCommitteeCache::new();
    let resolver =
        CommitteeResolver::anchor_with_cache(grpc_client(&cluster), genesis_committee(&cluster), cache.clone());

    let error = resolver.resolve(1).await.unwrap_err();

    assert!(matches!(
        error.kind,
        CommitteeResolutionErrorKind::TargetAheadOfNode { current_epoch: 0 }
    ));
    assert!(cache.is_empty().await);
}

#[tokio::test]
async fn trusted_node_resolution_does_not_write_to_an_anchor_cache() {
    let cluster = start_test_cluster().await;
    let cache = MemoryCommitteeCache::new();
    let resolver = CommitteeResolver::node(grpc_client(&cluster));

    let resolved = resolver
        .resolve(0)
        .await
        .expect("trusted node must return its genesis committee");

    assert_eq!(resolved, *cluster.committee());
    assert!(cache.is_empty().await);
}
