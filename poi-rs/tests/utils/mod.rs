// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Each integration-test file is compiled as a separate crate, so helpers used
// by sibling test crates otherwise appear unused.
#![allow(dead_code)]

use std::fs::File;

use iota_config::IOTA_GENESIS_FILENAME;
use iota_config::genesis::Genesis;
use iota_grpc_client::Client as GrpcClient;
use iota_grpc_client::ReadMask;
use iota_grpc_client::read_mask_fields::TransactionField;
use iota_sdk_types::{ObjectReference, TransactionDigest};
use iota_types::committee::Committee;
use iota_types::digests::ChainIdentifier;
use iota_types::iota_system_state::{IotaSystemStateTrait, get_iota_system_state};
use iota_types::transaction::TransactionData;
use test_cluster::{TestCluster, TestClusterBuilder};

pub mod proofs;
pub mod sources;

pub fn committee_at(epoch: u64) -> Committee {
    let (committee, _) = Committee::new_simple_test_committee();
    Committee::new(epoch, committee.voting_rights.iter().cloned().collect())
}

pub struct CheckpointedTransfer {
    pub digest: TransactionDigest,
    pub gas_object: ObjectReference,
}

pub struct CheckpointedStaking {
    pub digest: TransactionDigest,
    pub gas_object: ObjectReference,
}

pub struct CheckpointedObjectTransfer {
    pub digest: TransactionDigest,
    pub objects: [ObjectReference; 2],
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

async fn execute_transaction(cluster: &TestCluster, transaction: &TransactionData) -> TransactionDigest {
    let signed_transaction = cluster.wallet.sign_transaction(transaction);
    let digest = *signed_transaction.digest();
    let response = grpc_client(cluster)
        .execute_transaction(
            signed_transaction.into(),
            Some(ReadMask::from(&[
                TransactionField::EFFECTS,
                TransactionField::CHECKPOINT,
            ])),
            Some(30_000),
        )
        .await
        .expect("transaction must execute and be checkpointed");
    let transaction = response.body();
    let effects = transaction
        .effects()
        .expect("transaction response must include effects")
        .effects()
        .expect("transaction effects must decode");
    assert!(effects.as_v1().status.is_success(), "transaction must succeed");
    transaction
        .checkpoint_sequence_number()
        .expect("transaction response must include its checkpoint");

    digest
}

pub async fn transfer_tx(cluster: &TestCluster) -> CheckpointedTransfer {
    let builder = cluster.test_transaction_builder().await;
    let gas_object = builder.gas_object();
    let transaction = builder.transfer_iota(Some(1), cluster.get_address_1()).build();
    let digest = execute_transaction(cluster, &transaction).await;
    let gas_object = cluster
        .wallet
        .get_object_ref(gas_object.object_id)
        .await
        .expect("mutated gas object must be available");

    CheckpointedTransfer { digest, gas_object }
}

pub async fn object_transfer_tx(cluster: &TestCluster) -> CheckpointedObjectTransfer {
    let (sender, mut coins) = cluster
        .wallet
        .get_one_account()
        .await
        .expect("test cluster must contain a funded account");
    let gas = coins.pop().expect("funded account must have a gas coin");
    let object = coins.pop().expect("funded account must have an object to transfer");
    let gas_object_id = gas.object_id;
    let transferred_object_id = object.object_id;
    let transaction = cluster
        .test_transaction_builder_with_gas_object(sender, gas)
        .await
        .transfer(object, cluster.get_address_1())
        .build();
    let digest = execute_transaction(cluster, &transaction).await;
    let gas_object = cluster
        .wallet
        .get_object_ref(gas_object_id)
        .await
        .expect("mutated gas object must be available");
    let transferred_object = cluster
        .wallet
        .get_object_ref(transferred_object_id)
        .await
        .expect("transferred object must be available");

    CheckpointedObjectTransfer {
        digest,
        objects: [gas_object, transferred_object],
    }
}

pub async fn staking_tx(cluster: &TestCluster) -> CheckpointedStaking {
    let (sender, mut coins) = cluster
        .wallet
        .get_one_account()
        .await
        .expect("test cluster must contain a funded account");
    let gas = coins.pop().expect("funded account must have a gas coin");
    let stake = coins.pop().expect("funded account must have a stake coin");
    let gas_object_id = gas.object_id;
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
    let digest = execute_transaction(cluster, &transaction).await;
    let gas_object = cluster
        .wallet
        .get_object_ref(gas_object_id)
        .await
        .expect("mutated gas object must be available");

    CheckpointedStaking { digest, gas_object }
}

pub fn genesis_committee(cluster: &TestCluster) -> Committee {
    let genesis_path = cluster.swarm.dir().join(IOTA_GENESIS_FILENAME);
    let genesis = File::open(genesis_path).expect("test cluster genesis blob must be available");

    committee_from_genesis(genesis).expect("test cluster genesis committee must be extractable")
}

pub fn committee_from_genesis(genesis: impl std::io::Read) -> Result<Committee, ()> {
    let genesis: iota_config::genesis::Genesis = bcs::from_reader(genesis).map_err(|_| ())?;
    let system_state = get_iota_system_state(&genesis.objects()).map_err(|_| ())?;

    Ok(system_state.get_current_epoch_committee().committee().clone())
}

pub fn genesis_chain_identifier(cluster: &TestCluster) -> ChainIdentifier {
    let genesis_path = cluster.swarm.dir().join(IOTA_GENESIS_FILENAME);
    let genesis = Genesis::load(genesis_path).expect("test cluster genesis blob must load");

    ChainIdentifier::from(*genesis.checkpoint().digest())
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
