// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_types::{
    base_types::dbg_object_id, committee::Committee, effects::TransactionEvents, event::EventID,
    messages_checkpoint::CheckpointContents, object::Object,
};
use poi_rs::{ProofTargets, ProofVerifier, VerifyErrorKind};
use utils::proofs::{
    end_of_epoch_data, event, execution_data, next_epoch_committee, proof_with_events, proof_with_targets,
    valid_transaction_proof,
};

#[test]
fn valid_transaction_proof_is_accepted() {
    let (committee, proof) = valid_transaction_proof();

    let result = ProofVerifier::new(&committee).verify(&proof);

    assert!(result.is_ok());
}

#[test]
fn transaction_digest_must_match_the_effects() {
    let (committee, mut proof) = valid_transaction_proof();
    proof.transaction_proof.effects = execution_data().effects;

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::TransactionDigestMismatch));
}

#[test]
fn events_digest_must_match_the_effects() {
    let (committee, mut proof) = valid_transaction_proof();
    proof.transaction_proof.events = Some(TransactionEvents(Vec::new()));

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::EventsDigestMismatch));
}

#[test]
fn checkpoint_contents_must_match_the_signed_summary() {
    let (committee, mut proof) = valid_transaction_proof();
    let alternate = execution_data();
    proof.transaction_proof.checkpoint_contents =
        CheckpointContents::new_with_digests_only_for_tests([alternate.digests()]);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::CheckpointSummary { .. }));
}

#[test]
fn transaction_must_be_present_in_the_checkpoint() {
    let (committee, mut proof) = valid_transaction_proof();
    let alternate = execution_data();
    proof.transaction_proof.transaction = alternate.transaction;
    proof.transaction_proof.effects = alternate.effects;

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::TransactionNotInCheckpoint));
}

#[test]
fn committee_target_requires_end_of_epoch_data() {
    let (committee, _) = Committee::new_simple_test_committee();
    let target = next_epoch_committee(&committee);
    let (verifying_committee, proof) = proof_with_targets(ProofTargets::new().set_committee(target), None);

    let error = ProofVerifier::new(&verifying_committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::MissingEndOfEpochCommittee));
}

#[test]
fn committee_target_must_match_end_of_epoch_data() {
    let (actual, _) = Committee::new_simple_test_committee();
    let actual = next_epoch_committee(&actual);
    let (wrong, _) = Committee::new_simple_test_committee_of_size(5);
    let wrong = next_epoch_committee(&wrong);
    let targets = ProofTargets::new().set_committee(wrong);
    let (committee, proof) = proof_with_targets(targets, Some(end_of_epoch_data(&actual)));

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::CommitteeMismatch));
}

#[test]
fn object_target_must_match_its_reference() {
    let object = Object::immutable_for_testing();
    let mut object_ref = object.as_inner().object_ref();
    object_ref.object_id = dbg_object_id(42);
    let targets = ProofTargets::new().add_object(object_ref, object);
    let (committee, proof) = proof_with_targets(targets, None);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::ObjectReferenceMismatch));
}

#[test]
fn object_target_must_appear_in_the_transaction_effects() {
    let object = Object::immutable_for_testing();
    let object_ref = object.as_inner().object_ref();
    let targets = ProofTargets::new().add_object(object_ref, object);
    let (committee, proof) = proof_with_targets(targets, None);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::ObjectNotFound));
}

#[test]
fn event_target_must_match_the_packaged_event() {
    let packaged = event(vec![1, 2, 3]);
    let target = event(vec![9, 9, 9]);
    let (committee, transaction_digest, mut proof) = proof_with_events(TransactionEvents(vec![packaged]));
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: 0,
    };
    proof.target = ProofTargets::new().add_event(event_id, target);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::EventContentsMismatch));
}

#[test]
fn event_target_must_belong_to_the_proven_transaction() {
    let target = event(vec![1, 2, 3]);
    let (committee, _, mut proof) = proof_with_events(TransactionEvents(vec![target.clone()]));
    let event_id = EventID {
        tx_digest: iota_types::digests::TransactionDigest::new([0xff; 32]),
        event_seq: 0,
    };
    proof.target = ProofTargets::new().add_event(event_id, target);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(error.kind, VerifyErrorKind::EventTransactionMismatch));
}

#[test]
fn event_sequence_must_exist_in_the_transaction() {
    let target = event(vec![1, 2, 3]);
    let (committee, transaction_digest, mut proof) = proof_with_events(TransactionEvents(vec![target.clone()]));
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: 1,
    };
    proof.target = ProofTargets::new().add_event(event_id, target);

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();

    assert!(matches!(
        error.kind,
        VerifyErrorKind::EventSequenceOutOfBounds { sequence: 1 }
    ));
}
