// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofVerifier};

const COMMITTEE: &str = include_str!("fixtures/v1/committee.json");
const TRANSACTION: &str = include_str!("fixtures/v1/transaction.json");
const OBJECT: &str = include_str!("fixtures/v1/object.json");
const EVENT: &str = include_str!("fixtures/v1/event.json");

fn assert_fixture_round_trips_and_verifies(fixture: &str) -> Proof {
    let committee: Committee = serde_json::from_str(COMMITTEE).expect("committee fixture must deserialize");
    let proof = Proof::from_json_slice(fixture.as_bytes()).expect("proof fixture must deserialize");

    let _verified = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("proof fixture must verify offline");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&proof.to_json_vec().expect("proof fixture must serialize"))
            .expect("serialized proof must be valid JSON"),
        serde_json::from_str::<serde_json::Value>(fixture).expect("proof fixture must be valid JSON")
    );
    assert_eq!(proof.version(), 1);
    assert!(matches!(&proof, Proof::ProofV1(_)));

    proof
}

#[test]
fn transaction_fixture_round_trips_and_verifies() {
    let proof = assert_fixture_round_trips_and_verifies(TRANSACTION);

    assert!(proof.targets().transaction.is_some());
    assert!(proof.targets().objects.is_empty());
    assert!(proof.targets().events.is_empty());
}

#[test]
fn object_fixture_round_trips_and_verifies() {
    let proof = assert_fixture_round_trips_and_verifies(OBJECT);

    assert!(proof.targets().transaction.is_none());
    assert_eq!(proof.targets().objects.len(), 1);
    assert!(proof.targets().events.is_empty());
}

#[test]
fn event_fixture_round_trips_and_verifies() {
    let proof = assert_fixture_round_trips_and_verifies(EVENT);

    assert!(proof.targets().transaction.is_none());
    assert!(proof.targets().objects.is_empty());
    assert_eq!(proof.targets().events.len(), 1);
}

#[test]
fn unsupported_proof_variant_is_rejected_during_deserialization() {
    let mut fixture: serde_json::Value = serde_json::from_str(TRANSACTION).expect("proof fixture must be valid JSON");
    let proof = fixture
        .as_object_mut()
        .expect("proof fixture must be an object")
        .remove("ProofV1")
        .expect("proof fixture must contain ProofV1");
    let fixture = serde_json::json!({ "ProofV2": proof });
    let error = serde_json::from_value::<Proof>(fixture).expect_err("an unsupported proof variant must be rejected");

    assert!(error.to_string().contains("ProofV2"));
}
