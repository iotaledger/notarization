// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Each integration-test file is compiled as a separate crate, so helpers used
// by sibling test crates otherwise appear unused.
#![allow(dead_code)]

use iota_config::{IOTA_GENESIS_FILENAME, genesis::Genesis};
use iota_grpc_client::Client as GrpcClient;
use iota_types::{base_types::ObjectRef, committee::Committee, digests::TransactionDigest};
use test_cluster::{TestCluster, TestClusterBuilder};

pub mod proofs;

pub struct CheckpointedTransfer {
    pub digest: TransactionDigest,
    pub gas_object: ObjectRef,
}

pub async fn start_test_cluster() -> TestCluster {
    TestClusterBuilder::new()
        .with_num_validators(1)
        .with_fullnode_enable_grpc_api(true)
        .disable_fullnode_pruning()
        .build()
        .await
}

pub fn grpc_client(cluster: &TestCluster) -> GrpcClient {
    GrpcClient::new(cluster.grpc_url()).expect("test cluster gRPC client must connect")
}

pub async fn transfer_tx(cluster: &TestCluster) -> CheckpointedTransfer {
    let builder = cluster.test_transaction_builder().await;
    let gas_object = builder.gas_object();
    let transaction = builder.transfer_iota(Some(1), cluster.get_address_1()).build();
    let response = cluster.sign_and_execute_transaction(&transaction).await;
    let checkpoint = response.checkpoint.expect("transfer transaction must be checkpointed");
    cluster.wait_for_checkpoint(checkpoint, None).await;
    let gas_object = cluster
        .wallet
        .get_object_ref(gas_object.object_id)
        .await
        .expect("mutated gas object must be available");

    CheckpointedTransfer {
        digest: response.digest,
        gas_object,
    }
}

pub async fn staking_tx(cluster: &TestCluster) -> TransactionDigest {
    let (sender, mut coins) = cluster.wallet.get_one_account().await.unwrap();
    let gas = coins.pop().expect("funded account must have a gas coin");
    let stake = coins.pop().expect("funded account must have a stake coin");
    let validator = cluster
        .swarm
        .active_validators()
        .next()
        .expect("test cluster must have a validator")
        .config()
        .iota_address();
    let transaction = cluster
        .test_transaction_builder_with_gas_object(sender, gas)
        .await
        .call_staking(stake, validator)
        .build();
    let response = cluster.sign_and_execute_transaction(&transaction).await;
    let checkpoint = response.checkpoint.expect("staking transaction must be checkpointed");
    cluster.wait_for_checkpoint(checkpoint, None).await;

    response.digest
}

pub fn genesis_committee(cluster: &TestCluster) -> Committee {
    let genesis_path = cluster.swarm.dir().join(IOTA_GENESIS_FILENAME);
    Genesis::load(genesis_path)
        .expect("test cluster genesis blob must load")
        .committee()
        .expect("genesis blob must contain a committee")
}

pub async fn advance_to_epoch(cluster: &TestCluster, target_epoch: u64) -> Vec<Committee> {
    let mut committees = vec![cluster.committee().as_ref().clone()];

    for epoch in 1..=target_epoch {
        cluster.force_new_epoch().await;
        let committee = cluster.committee().as_ref().clone();
        assert_eq!(committee.epoch, epoch);
        committees.push(committee);
    }

    let _ = transfer_tx(cluster).await;

    committees
}
