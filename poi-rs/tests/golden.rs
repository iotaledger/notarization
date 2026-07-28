// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofVerifier, ProofVersion, VerifyErrorKind};

const COMMITTEE: &str = include_str!("fixtures/current/committee.json");
const TRANSACTION: &str = include_str!("fixtures/current/transaction.json");
const OBJECT: &str = include_str!("fixtures/current/object.json");
const EVENT: &str = include_str!("fixtures/current/event.json");

fn assert_current_format(fixture: &str) -> Proof {
    let committee: Committee = serde_json::from_str(COMMITTEE).expect("committee fixture must deserialize");
    let proof = Proof::from_json_slice(fixture.as_bytes()).expect("proof fixture must deserialize");

    ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("proof fixture must verify offline");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&proof.to_json_vec().expect("proof fixture must serialize"))
            .expect("serialized proof must be valid JSON"),
        serde_json::from_str::<serde_json::Value>(fixture).expect("proof fixture must be valid JSON")
    );
    assert_eq!(proof.version(), ProofVersion::CURRENT);

    proof
}

#[test]
fn current_transaction_fixture_remains_stable() {
    let proof = assert_current_format(TRANSACTION);

    assert!(proof.target().objects.is_empty());
    assert!(proof.target().events.is_empty());
}

#[test]
fn current_object_fixture_remains_stable() {
    let proof = assert_current_format(OBJECT);

    assert_eq!(proof.target().objects.len(), 1);
    assert!(proof.target().events.is_empty());
}

#[test]
fn current_event_fixture_remains_stable() {
    let proof = assert_current_format(EVENT);

    assert!(proof.target().objects.is_empty());
    assert_eq!(proof.target().events.len(), 1);
}

#[test]
fn unsupported_fixture_version_returns_the_version_number() {
    let committee: Committee = serde_json::from_str(COMMITTEE).expect("committee fixture must deserialize");
    let mut fixture: serde_json::Value = serde_json::from_str(TRANSACTION).expect("proof fixture must be valid JSON");
    fixture["version"] = serde_json::json!(2);
    let proof: Proof = serde_json::from_value(fixture).expect("unsupported proof version must deserialize");

    let error = ProofVerifier::new(&committee).verify(&proof).unwrap_err();
    let VerifyErrorKind::Version { source } = error.kind else {
        panic!("unsupported proof version must return a version error");
    };

    assert_eq!(source.version, 2);
}
