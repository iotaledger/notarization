// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::gas::GasCostSummary;
use iota_types::{
    base_types::ExecutionData,
    committee::Committee,
    digests::ChainIdentifier,
    effects::TransactionEvents,
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents, CheckpointSummary, FullCheckpointContents},
};
use poi_rs::{Error, Proof, ProofTargets, ProofVerifier, TransactionProof};

fn test_execution_data() -> ExecutionData {
    FullCheckpointContents::random_for_testing()
        .into_iter()
        .next()
        .expect("test checkpoint contents includes one transaction")
}

fn test_proof() -> (Committee, Proof) {
    let execution_data = test_execution_data();
    let checkpoint_contents = CheckpointContents::new_with_digests_only_for_tests([execution_data.digests()]);
    let checkpoint_summary = CheckpointSummary {
        epoch: 0,
        sequence_number: 0,
        network_total_transactions: checkpoint_contents.size() as u64,
        content_digest: *checkpoint_contents.digest(),
        previous_digest: None,
        epoch_rolling_gas_cost_summary: GasCostSummary::default(),
        timestamp_ms: 0,
        checkpoint_commitments: Vec::new(),
        end_of_epoch_data: None,
        version_specific_data: Vec::new(),
    };
    let (committee, keypairs) = Committee::new_simple_test_committee();
    let checkpoint_summary =
        CertifiedCheckpointSummary::new_from_keypairs_for_testing(checkpoint_summary, &keypairs, &committee);
    let chain = ChainIdentifier::from(*checkpoint_summary.digest());

    let proof = Proof::new(
        chain,
        ProofTargets::new(),
        checkpoint_summary,
        TransactionProof::new(
            checkpoint_contents,
            execution_data.transaction,
            execution_data.effects,
            None,
        ),
    );

    (committee, proof)
}

#[test]
fn verifier_accepts_valid_transaction_proof() {
    let (committee, proof) = test_proof();

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(result.is_ok());
}

#[test]
fn verifier_rejects_transaction_digest_mismatch() {
    let (committee, mut proof) = test_proof();
    proof.transaction_proof.effects = test_execution_data().effects;

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(Error::TransactionDigestMismatch)));
}

#[test]
fn verifier_rejects_events_digest_mismatch() {
    let (committee, mut proof) = test_proof();
    proof.transaction_proof.events = Some(TransactionEvents(Vec::new()));

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(Error::EventsDigestMismatch)));
}
