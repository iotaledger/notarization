// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::{CheckpointContents, CheckpointSummary, Event, TransactionDigest, gas::GasCostSummary};
use iota_types::{
    base_types::ExecutionData,
    committee::Committee,
    digests::ChainIdentifier,
    effects::{TestEffectsBuilder, TransactionEvents},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContentsExt, FullCheckpointContents},
    sdk_types::{Address, Identifier, ObjectId, StructTag},
};
use poi_rs::{Proof, ProofTargets, TransactionProof};

pub fn execution_data() -> ExecutionData {
    FullCheckpointContents::random_for_testing()
        .into_iter()
        .next()
        .expect("test checkpoint contents must include a transaction")
}

fn signed_checkpoint(contents: &CheckpointContents) -> (Committee, CertifiedCheckpointSummary) {
    let summary = CheckpointSummary {
        epoch: 0,
        sequence_number: 0,
        network_total_transactions: contents.len() as u64,
        content_digest: contents.digest(),
        previous_digest: None,
        epoch_rolling_gas_cost_summary: GasCostSummary::default(),
        timestamp_ms: 0,
        checkpoint_commitments: Vec::new(),
        end_of_epoch_data: None,
        version_specific_data: Vec::new(),
    };
    let (committee, keypairs) = Committee::new_simple_test_committee();
    let summary = CertifiedCheckpointSummary::new_from_keypairs_for_testing(summary, &keypairs, &committee);

    (committee, summary)
}

pub fn valid_transaction_proof() -> (Committee, Proof) {
    proof_with_targets(ProofTargets::new())
}

pub fn proof_with_targets(targets: ProofTargets) -> (Committee, Proof) {
    let execution = execution_data();
    let contents = CheckpointContents::new_with_digests_only_for_tests([execution.digests()]);
    let (committee, summary) = signed_checkpoint(&contents);
    let chain = ChainIdentifier::from(*summary.digest());
    let proof = Proof::new(
        chain,
        targets,
        summary,
        TransactionProof::new(contents, execution.transaction, execution.effects, None),
    );

    (committee, proof)
}

pub fn proof_with_events(events: TransactionEvents) -> (Committee, TransactionDigest, Proof) {
    let mut execution = execution_data();
    let transaction_digest = *execution.transaction.digest();
    execution.effects = TestEffectsBuilder::new(execution.transaction.data())
        .with_events_digest(events.digest())
        .build();
    let contents = CheckpointContents::new_with_digests_only_for_tests([execution.digests()]);
    let (committee, summary) = signed_checkpoint(&contents);
    let chain = ChainIdentifier::from(*summary.digest());
    let proof = Proof::new(
        chain,
        ProofTargets::new(),
        summary,
        TransactionProof::new(contents, execution.transaction, execution.effects, Some(events)),
    );

    (committee, transaction_digest, proof)
}

pub fn event(contents: Vec<u8>) -> Event {
    Event {
        package_id: ObjectId::SYSTEM,
        module: Identifier::IOTA_SYSTEM_MODULE,
        sender: Address::SYSTEM,
        type_: StructTag::new(
            Address::SYSTEM,
            Identifier::IOTA_SYSTEM_MODULE,
            Identifier::SYSTEM_EPOCH_INFO_EVENT,
            Vec::new(),
        ),
        contents,
    }
}
