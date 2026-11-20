// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::{Proof, ProofV2, ProofVerifier};

const COMMITTEE: &str = include_str!("fixtures/v1/committee.json");
const PROLOGUE_COMMITTEE: &str = include_str!("fixtures/v1/consensus_commit_prologue_committee.json");

const V1_TRANSACTION: &str = include_str!("fixtures/v1/transaction.json");
const V1_OBJECT: &str = include_str!("fixtures/v1/object.json");
const V1_EVENT: &str = include_str!("fixtures/v1/event.json");
const V1_PROLOGUE: &str = include_str!("fixtures/v1/consensus_commit_prologue.json");

const V2_TRANSACTION: &str = include_str!("fixtures/v2/transaction.json");
const V2_OBJECT: &str = include_str!("fixtures/v2/object.json");
const V2_EVENT: &str = include_str!("fixtures/v2/event.json");
const V2_PROLOGUE: &str = include_str!("fixtures/v2/consensus_commit_prologue.json");

/// Identifiers that `iota-sdk-types` renamed in 1.0.0-rc.1, as (version 1 spelling, version 2 spelling).
const RENAMED: [(&str, &str); 5] = [
    ("\"CancelledTransactions\"", "\"CanceledTransactions\""),
    ("\"cancelled_transactions\"", "\"canceled_transactions\""),
    ("\"Cancelled\"", "\"Canceled\""),
    (
        "\"ExecutionCancelledDueToSharedObjectCongestionV2\"",
        "\"ExecutionCanceledDueToSharedObjectCongestionV2\"",
    ),
    ("\"MoveAuthenticationError\"", "\"MoveAuthentication\""),
];

fn assert_fixture_round_trips_and_verifies(fixture: &str, committee: &str, version: u16) -> Proof {
    let committee: Committee = serde_json::from_str(committee).expect("committee fixture must deserialize");
    let proof = Proof::from_json_slice(fixture.as_bytes()).expect("proof fixture must deserialize");

    let _verified = ProofVerifier::new(&committee)
        .verify(&proof)
        .expect("proof fixture must verify offline");
    // Same structure and the same key order: the compact form of the fixture.
    let fixture_value = serde_json::from_str::<serde_json::Value>(fixture).expect("proof fixture must be valid JSON");
    assert_eq!(
        proof.to_json_vec().expect("proof fixture must serialize"),
        serde_json::to_vec(&fixture_value).expect("proof fixture must serialize as JSON")
    );
    assert_eq!(proof.version(), version);
    match version {
        1 => assert!(matches!(&proof, Proof::ProofV1(_))),
        2 => assert!(matches!(&proof, Proof::ProofV2(_))),
        _ => unreachable!("unknown fixture version"),
    }

    proof
}

#[test]
fn transaction_fixtures_round_trip_and_verify() {
    for (fixture, version) in [(V1_TRANSACTION, 1), (V2_TRANSACTION, 2)] {
        let proof = assert_fixture_round_trips_and_verifies(fixture, COMMITTEE, version);

        assert!(proof.targets().transaction.is_some());
        assert!(proof.targets().objects.is_empty());
        assert!(proof.targets().events.is_empty());
    }
}

#[test]
fn object_fixtures_round_trip_and_verify() {
    for (fixture, version) in [(V1_OBJECT, 1), (V2_OBJECT, 2)] {
        let proof = assert_fixture_round_trips_and_verifies(fixture, COMMITTEE, version);

        assert!(proof.targets().transaction.is_none());
        assert_eq!(proof.targets().objects.len(), 1);
        assert!(proof.targets().events.is_empty());
    }
}

#[test]
fn event_fixtures_round_trip_and_verify() {
    for (fixture, version) in [(V1_EVENT, 1), (V2_EVENT, 2)] {
        let proof = assert_fixture_round_trips_and_verifies(fixture, COMMITTEE, version);

        assert!(proof.targets().transaction.is_none());
        assert!(proof.targets().objects.is_empty());
        assert_eq!(proof.targets().events.len(), 1);
    }
}

#[test]
fn consensus_commit_prologue_fixtures_round_trip_and_verify() {
    for (fixture, version) in [(V1_PROLOGUE, 1), (V2_PROLOGUE, 2)] {
        let proof = assert_fixture_round_trips_and_verifies(fixture, PROLOGUE_COMMITTEE, version);

        assert!(proof.targets().transaction.is_some());
    }
}

#[test]
fn version_1_proofs_keep_the_spelling_they_were_written_with() {
    let proof = Proof::from_json_slice(V1_PROLOGUE.as_bytes()).expect("proof fixture must deserialize");
    let json = String::from_utf8(proof.to_json_vec().expect("proof must serialize")).expect("JSON is UTF-8");

    for (v1, v2) in RENAMED {
        assert!(json.contains(v1), "version 1 JSON must contain {v1}");
        assert!(!json.contains(v2), "version 1 JSON must not contain {v2}");
    }
}

#[test]
fn version_1_proofs_convert_to_the_version_2_fixture() {
    let Proof::ProofV1(proof) = Proof::from_json_slice(V1_PROLOGUE.as_bytes()).expect("proof fixture must deserialize")
    else {
        panic!("the fixture must be a version 1 proof");
    };
    let converted = Proof::from(ProofV2::from(proof));
    let json = String::from_utf8(converted.to_json_vec().expect("proof must serialize")).expect("JSON is UTF-8");

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).expect("converted proof must be valid JSON"),
        serde_json::from_str::<serde_json::Value>(V2_PROLOGUE).expect("proof fixture must be valid JSON")
    );
    for (v1, v2) in RENAMED {
        assert!(json.contains(v2), "version 2 JSON must contain {v2}");
        assert!(!json.contains(v1), "version 2 JSON must not contain {v1}");
    }
}

#[test]
fn unsupported_proof_variant_is_rejected_during_deserialization() {
    let mut fixture: serde_json::Value =
        serde_json::from_str(V2_TRANSACTION).expect("proof fixture must be valid JSON");
    let proof = fixture
        .as_object_mut()
        .expect("proof fixture must be an object")
        .remove("ProofV2")
        .expect("proof fixture must contain ProofV2");
    let fixture = serde_json::json!({ "ProofV3": proof });
    let error = serde_json::from_value::<Proof>(fixture).expect_err("an unsupported proof variant must be rejected");

    assert!(error.to_string().contains("ProofV3"));
}
