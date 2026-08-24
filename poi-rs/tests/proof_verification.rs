// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod utils;

use iota_sdk_types::CheckpointContents;
use iota_types::effects::TransactionEvents;
use iota_types::event::EventID;
use iota_types::messages_checkpoint::CheckpointContentsExt;
use iota_types::object::Object;
use poi_rs::{Proof, ProofTargets, ProofV1, ProofVerifier, VerifyErrorKind};
use utils::proofs::{event, execution_data, proof_with_events, proof_with_targets, valid_transaction_proof};

fn proof_v1_mut(proof: &mut Proof) -> &mut ProofV1 {
    match proof {
        Proof::ProofV1(proof) => proof,
        _ => panic!("test helper expected a version 1 proof"),
    }
}

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
    proof_v1_mut(&mut proof).transaction_proof.effects = execution_data().effects;

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("mismatched transaction effects must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::TransactionDigestMismatch));
}

#[test]
fn events_digest_must_match_the_effects() {
    let (committee, mut proof) = valid_transaction_proof();
    proof_v1_mut(&mut proof).transaction_proof.events = Some(TransactionEvents(Vec::new()));

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("mismatched transaction events must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::EventsDigestMismatch));
}

#[test]
fn checkpoint_contents_must_match_the_signed_summary() {
    let (committee, mut proof) = valid_transaction_proof();
    let alternate = execution_data();
    proof_v1_mut(&mut proof).checkpoint_contents =
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
    {
        let proof = proof_v1_mut(&mut proof);
        proof.transaction_proof.transaction = alternate.transaction;
        proof.transaction_proof.effects = alternate.effects;
    }

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("a transaction outside the checkpoint must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::TransactionNotInCheckpoint));
}

#[test]
fn object_target_must_appear_in_the_transaction_effects() {
    let object = Object::immutable_for_testing();
    let targets = ProofTargets::new().add_object(object);
    let (committee, proof) = proof_with_targets(targets);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an object absent from the transaction effects must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::ObjectNotFound));
}

#[test]
fn event_target_must_belong_to_the_proven_transaction() {
    let target = event(vec![1, 2, 3]);
    let (committee, _, mut proof) = proof_with_events(TransactionEvents(vec![target]));
    let event_id = EventID {
        tx_digest: iota_sdk_types::TransactionDigest::new([0xff; 32]),
        event_seq: 0,
    };
    proof_v1_mut(&mut proof).targets = ProofTargets::new().add_event(event_id);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an event from another transaction must be rejected");

    assert!(matches!(error.kind, VerifyErrorKind::EventTransactionMismatch));
}

#[test]
fn event_sequence_must_exist_in_the_transaction() {
    let target = event(vec![1, 2, 3]);
    let (committee, transaction_digest, mut proof) = proof_with_events(TransactionEvents(vec![target]));
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: 1,
    };
    proof_v1_mut(&mut proof).targets = ProofTargets::new().add_event(event_id);

    let error = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect_err("an event sequence outside the transaction must be rejected");

    assert!(matches!(
        error.kind,
        VerifyErrorKind::EventSequenceOutOfBounds { sequence: 1 }
    ));
}
