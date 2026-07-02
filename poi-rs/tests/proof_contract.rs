// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use poi_rs::{Error, Proof, ProofVersion};

#[test]
fn current_proof_format_version_is_one() {
    assert_eq!(ProofVersion::CURRENT.value(), 1);
    assert_eq!(
        ProofVersion::new(ProofVersion::CURRENT.value()).unwrap(),
        ProofVersion::CURRENT
    );
}
