// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_grpc_client::Client as GrpcClient;
use iota_types::committee::Committee;
use poi_rs::{CommitteeResolutionErrorKind, CommitteeResolver};

fn committee_at(epoch: u64) -> Committee {
    let (committee, _) = Committee::new_simple_test_committee();
    Committee::new(epoch, committee.voting_rights.iter().cloned().collect())
}

fn disconnected_client() -> GrpcClient {
    GrpcClient::new("http://127.0.0.1:1").expect("create lazy gRPC client")
}

#[tokio::test]
async fn anchor_mode_returns_the_trusted_committee_for_its_epoch() {
    let trusted_committee = committee_at(7);
    let resolver = CommitteeResolver::anchor(disconnected_client(), trusted_committee.clone());

    let resolved = resolver.resolve(7).await.unwrap();

    assert_eq!(resolved, trusted_committee);
}

#[tokio::test]
async fn anchor_mode_rejects_an_epoch_before_the_trust_anchor() {
    let resolver = CommitteeResolver::anchor(disconnected_client(), committee_at(7));

    let error = resolver.resolve(6).await.unwrap_err();

    assert_eq!(error.target_epoch, 6);
    assert!(matches!(
        error.kind,
        CommitteeResolutionErrorKind::TargetBeforeAnchor { anchor_epoch: 7 }
    ));
}

#[tokio::test]
async fn node_mode_has_an_explicit_constructor() {
    let _resolver = CommitteeResolver::node(disconnected_client());
}
