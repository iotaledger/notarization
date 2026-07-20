// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofVerifier, ProofVersion, VerifyErrorKind};

const COMMITTEE: &str = include_str!("fixtures/v1/committee.json");
const TRANSACTION: &str = include_str!("fixtures/v1/transaction.json");
const OBJECT: &str = include_str!("fixtures/v1/object.json");
const EVENT: &str = include_str!("fixtures/v1/event.json");

fn assert_version_one_compatibility(fixture: &str) -> Proof {
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
fn version_one_transaction_fixture_remains_compatible() {
    let proof = assert_version_one_compatibility(TRANSACTION);

    assert!(proof.target().objects.is_empty());
    assert!(proof.target().events.is_empty());
}

#[test]
fn version_one_object_fixture_remains_compatible() {
    let proof = assert_version_one_compatibility(OBJECT);

    assert_eq!(proof.target().objects.len(), 1);
    assert!(proof.target().events.is_empty());
}

#[test]
fn version_one_event_fixture_remains_compatible() {
    let proof = assert_version_one_compatibility(EVENT);

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
