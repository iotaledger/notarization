// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofVerifier, ProofVersion, TransactionProof};

fn proof_transaction_proof_is_required(proof: Proof) -> TransactionProof {
    proof.transaction_proof
}

#[test]
fn current_proof_format_version_is_one() {
    assert_eq!(ProofVersion::CURRENT.value(), 1);
    assert_eq!(
        ProofVersion::new(ProofVersion::CURRENT.value()).unwrap(),
        ProofVersion::CURRENT
    );
}

#[test]
fn proof_requires_transaction_proof() {
    let transaction_proof_field: fn(Proof) -> TransactionProof = proof_transaction_proof_is_required;
    let _ = transaction_proof_field;
}

#[test]
fn proof_verifier_is_the_public_verification_entrypoint() {
    let (committee, _) = Committee::new_simple_test_committee();
    let verifier = ProofVerifier::new(&committee);
    let verify_method = ProofVerifier::verify;

    assert_eq!(verifier.committee().epoch, committee.epoch);
    let _ = verify_method;
}
