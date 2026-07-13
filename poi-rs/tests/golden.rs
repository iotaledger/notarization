// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofVerifier, ProofVersion, VerifyErrorKind};

const COMMITTEE: &str = include_str!("fixtures/v1/committee.json");
const TRANSACTION_PROOF: &str = include_str!("fixtures/v1/transaction.json");
const OBJECT_PROOF: &str = include_str!("fixtures/v1/object.json");
const EVENT_PROOF: &str = include_str!("fixtures/v1/event.json");

fn verify_fixture(fixture: &str) -> Proof {
    let committee: Committee = serde_json::from_str(COMMITTEE).expect("version 1 committee fixture must deserialize");
    let proof: Proof = serde_json::from_str(fixture).expect("version 1 proof fixture must deserialize");

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("version 1 proof fixture must verify offline");

    let fixture_value: serde_json::Value =
        serde_json::from_str(fixture).expect("version 1 proof fixture must contain valid JSON");
    let serialized_value = serde_json::to_value(&proof).expect("version 1 proof fixture must serialize");
    assert_eq!(serialized_value, fixture_value);
    assert_eq!(proof.version(), ProofVersion::CURRENT);

    proof
}

#[test]
fn transaction_fixture_verifies_offline() {
    let proof = verify_fixture(TRANSACTION_PROOF);

    assert!(proof.target().objects.is_empty());
    assert!(proof.target().events.is_empty());
    assert!(proof.target().committee.is_none());
}

#[test]
fn object_fixture_verifies_offline() {
    let proof = verify_fixture(OBJECT_PROOF);

    assert_eq!(proof.target().objects.len(), 1);
    assert!(proof.target().events.is_empty());
    assert!(proof.target().committee.is_none());
}

#[test]
fn event_fixture_verifies_offline() {
    let proof = verify_fixture(EVENT_PROOF);

    assert!(proof.target().objects.is_empty());
    assert_eq!(proof.target().events.len(), 1);
    assert!(proof.target().committee.is_none());
}

#[test]
fn unsupported_fixture_version_returns_a_clear_error() {
    let committee: Committee = serde_json::from_str(COMMITTEE).expect("version 1 committee fixture must deserialize");
    let mut fixture: serde_json::Value =
        serde_json::from_str(TRANSACTION_PROOF).expect("version 1 transaction fixture must contain valid JSON");
    fixture["version"] = serde_json::json!(2);
    let proof: Proof = serde_json::from_value(fixture).expect("unsupported proof version must deserialize");

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();
    let VerifyErrorKind::Version { source } = error.kind else {
        panic!("unsupported proof version must return a version error");
    };

    assert_eq!(source.version, 2);
    assert_eq!(
        source.to_string(),
        "unsupported Proof of Inclusion proof format version: 2"
    );
}
