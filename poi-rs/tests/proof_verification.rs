// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_sdk_types::CheckpointContents;
use iota_types::{
    base_types::dbg_object_id, effects::TransactionEvents, event::EventID, messages_checkpoint::CheckpointContentsExt,
    object::Object,
};
use poi_rs::{ProofTargets, ProofVerifier, VerifyErrorKind};
use utils::proofs::{event, execution_data, proof_with_events, proof_with_targets, valid_transaction_proof};

#[test]
fn valid_transaction_proof_is_accepted() {
    let (committee, proof) = valid_transaction_proof();

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("a valid transaction proof must verify");
}

#[test]
fn transaction_digest_must_match_the_effects() {
    let (committee, mut proof) = valid_transaction_proof();
    proof.transaction_proof.effects = execution_data().effects;

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("mismatched transaction effects must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::TransactionDigestMismatch));
}

#[test]
fn events_digest_must_match_the_effects() {
    let (committee, mut proof) = valid_transaction_proof();
    proof.transaction_proof.events = Some(TransactionEvents(Vec::new()));

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("mismatched transaction events must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::EventsDigestMismatch));
}

#[test]
fn checkpoint_contents_must_match_the_signed_summary() {
    let (committee, mut proof) = valid_transaction_proof();
    let alternate = execution_data();
    proof.transaction_proof.checkpoint_contents =
        CheckpointContents::new_with_digests_only_for_tests([alternate.digests()]);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("checkpoint contents outside the signed summary must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::CheckpointSummary { .. }));
}

#[test]
fn transaction_must_be_present_in_the_checkpoint() {
    let (committee, mut proof) = valid_transaction_proof();
    let alternate = execution_data();
    proof.transaction_proof.transaction = alternate.transaction;
    proof.transaction_proof.effects = alternate.effects;

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("a transaction outside the checkpoint must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::TransactionNotInCheckpoint));
}

#[test]
fn object_target_must_match_its_reference() {
    let object = Object::immutable_for_testing();
    let mut object_ref = object.as_inner().object_ref();
    object_ref.object_id = dbg_object_id(42);
    let targets = ProofTargets::new().add_object(object_ref, object);
    let (committee, proof) = proof_with_targets(targets);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an object that does not match its reference must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::ObjectReferenceMismatch));
}

#[test]
fn object_target_must_appear_in_the_transaction_effects() {
    let object = Object::immutable_for_testing();
    let object_ref = object.as_inner().object_ref();
    let targets = ProofTargets::new().add_object(object_ref, object);
    let (committee, proof) = proof_with_targets(targets);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an object absent from the transaction effects must be rejected");

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

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an event that does not match the packaged event must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::EventContentsMismatch));
}

#[test]
fn event_target_must_belong_to_the_proven_transaction() {
    let target = event(vec![1, 2, 3]);
    let (committee, _, mut proof) = proof_with_events(TransactionEvents(vec![target.clone()]));
    let event_id = EventID {
        tx_digest: iota_sdk_types::TransactionDigest::new([0xff; 32]),
        event_seq: 0,
    };
    proof.target = ProofTargets::new().add_event(event_id, target);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an event from another transaction must be rejected");

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

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an event sequence outside the transaction must be rejected");

    assert!(matches!(
        error.kind,
        VerifyErrorKind::EventSequenceOutOfBounds { sequence: 1 }
    ));
}
