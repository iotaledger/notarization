// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::gas::GasCostSummary;
use iota_types::{
    base_types::{ExecutionData, dbg_object_id},
    committee::Committee,
    digests::ChainIdentifier,
    effects::TransactionEvents,
    messages_checkpoint::{
        CertifiedCheckpointSummary, CheckpointContents, CheckpointSummary, EndOfEpochData, FullCheckpointContents,
    },
    object::Object,
};
use poi_rs::{Proof, ProofTargets, ProofVerifier, TransactionProof, VerifyErrorKind};

fn test_execution_data() -> ExecutionData {
    FullCheckpointContents::random_for_testing()
        .into_iter()
        .next()
        .expect("test checkpoint contents includes one transaction")
}

fn sign_checkpoint_summary(
    checkpoint_contents: &CheckpointContents,
    end_of_epoch_data: Option<EndOfEpochData>,
) -> (Committee, CertifiedCheckpointSummary) {
    let checkpoint_summary = CheckpointSummary {
        epoch: 0,
        sequence_number: 0,
        network_total_transactions: checkpoint_contents.size() as u64,
        content_digest: *checkpoint_contents.digest(),
        previous_digest: None,
        epoch_rolling_gas_cost_summary: GasCostSummary::default(),
        timestamp_ms: 0,
        checkpoint_commitments: Vec::new(),
        end_of_epoch_data,
        version_specific_data: Vec::new(),
    };
    let (committee, keypairs) = Committee::new_simple_test_committee();
    let checkpoint_summary =
        CertifiedCheckpointSummary::new_from_keypairs_for_testing(checkpoint_summary, &keypairs, &committee);

    (committee, checkpoint_summary)
}

fn test_proof() -> (Committee, Proof) {
    test_proof_with_targets_and_end_of_epoch_data(ProofTargets::new(), None)
}

fn test_proof_with_targets_and_end_of_epoch_data(
    targets: ProofTargets,
    end_of_epoch_data: Option<EndOfEpochData>,
) -> (Committee, Proof) {
    let execution_data = test_execution_data();
    let checkpoint_contents = CheckpointContents::new_with_digests_only_for_tests([execution_data.digests()]);
    let (committee, checkpoint_summary) = sign_checkpoint_summary(&checkpoint_contents, end_of_epoch_data);
    let chain = ChainIdentifier::from(*checkpoint_summary.digest());

    let proof = Proof::new(
        chain,
        targets,
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

fn epoch_one_committee(committee: &Committee) -> Committee {
    Committee::new(1, committee.voting_rights.iter().cloned().collect())
}

fn end_of_epoch_data_for(committee: &Committee) -> EndOfEpochData {
    EndOfEpochData {
        next_epoch_committee: committee.voting_rights.clone(),
        next_epoch_protocol_version: 1.into(),
        epoch_commitments: Vec::new(),
        epoch_supply_change: 0,
    }
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

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::TransactionDigestMismatch)));
}

#[test]
fn verifier_rejects_events_digest_mismatch() {
    let (committee, mut proof) = test_proof();
    proof.transaction_proof.events = Some(TransactionEvents(Vec::new()));

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::EventsDigestMismatch)));
}

#[test]
fn verifier_rejects_checkpoint_contents_mismatch() {
    let (committee, mut proof) = test_proof();
    let alternate_execution_data = test_execution_data();
    proof.transaction_proof.checkpoint_contents =
        CheckpointContents::new_with_digests_only_for_tests([alternate_execution_data.digests()]);

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::CheckpointSummary { .. })));
}

#[test]
fn verifier_rejects_transaction_not_in_checkpoint() {
    let (committee, mut proof) = test_proof();
    let alternate_execution_data = test_execution_data();
    proof.transaction_proof.transaction = alternate_execution_data.transaction;
    proof.transaction_proof.effects = alternate_execution_data.effects;

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::TransactionNotInCheckpoint)));
}

#[test]
fn verifier_rejects_missing_end_of_epoch_committee() {
    let (committee, _) = Committee::new_simple_test_committee();
    let expected_committee = epoch_one_committee(&committee);
    let (verifying_committee, proof) =
        test_proof_with_targets_and_end_of_epoch_data(ProofTargets::new().set_committee(expected_committee), None);

    let result = ProofVerifier::new(&verifying_committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::MissingEndOfEpochCommittee)));
}

#[test]
fn verifier_rejects_committee_mismatch() {
    let (actual_next_committee, _) = Committee::new_simple_test_committee();
    let actual_next_committee = epoch_one_committee(&actual_next_committee);
    let (wrong_next_committee, _) = Committee::new_simple_test_committee_of_size(5);
    let wrong_next_committee = epoch_one_committee(&wrong_next_committee);
    let (verifying_committee, proof) = test_proof_with_targets_and_end_of_epoch_data(
        ProofTargets::new().set_committee(wrong_next_committee),
        Some(end_of_epoch_data_for(&actual_next_committee)),
    );

    let result = ProofVerifier::new(&verifying_committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::CommitteeMismatch)));
}

#[test]
fn verifier_rejects_object_reference_mismatch() {
    let object = Object::immutable_for_testing();
    let mut wrong_object_ref = object.compute_object_reference();
    wrong_object_ref.object_id = dbg_object_id(42);
    let (committee, proof) =
        test_proof_with_targets_and_end_of_epoch_data(ProofTargets::new().add_object(wrong_object_ref, object), None);

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::ObjectReferenceMismatch)));
}

#[test]
fn verifier_rejects_object_not_found_in_transaction_effects() {
    let object = Object::immutable_for_testing();
    let object_ref = object.compute_object_reference();
    let (committee, proof) =
        test_proof_with_targets_and_end_of_epoch_data(ProofTargets::new().add_object(object_ref, object), None);

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(matches!(result, Err(error) if matches!(error.kind, VerifyErrorKind::ObjectNotFound)));
}
