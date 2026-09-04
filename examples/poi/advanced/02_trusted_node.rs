// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Verify a Proof Using a Trusted Node
//!
//! Trusted-node committee resolution accepts the committee reported by the
//! connected node without authenticating its lineage from genesis. It is
//! appropriate only when that node is inside the verifier's trust boundary.
//!
//! This example can run against any network, but the selected gRPC endpoint
//! must be operated by a party the verifier trusts.

use anyhow::{Context, Result};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates trusted-node committee resolution against a trusted endpoint.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion Advanced: Trusted-Node Resolution ===\n");

    let context = prepare_poi_example().await?;

    let transaction_digest = context
        .create_notarization("PoI trusted-node example")
        .await?
        .transaction_digest;
    let client = &context.poi_client;
    let proof = client
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .context("failed to construct the transaction proof")?;

    println!("Network:              {}", context.network_alias);
    println!("Committee resolution: trusted node");
    println!("Transaction target:   {transaction_digest}\n");

    let verified = client
        .verifier(CommitteeResolution::TrustedNode)
        .verify(&proof)
        .await
        .context("trusted-node proof verification failed")?;

    println!("Transaction proof verified successfully.");
    println!("  authenticated checkpoint: {}", verified.checkpoint_sequence_number());
    println!("  authenticated transaction: {}", verified.transaction_digest());
    println!("The selected node supplied the committee and is part of the trust boundary.");

    Ok(())
}
