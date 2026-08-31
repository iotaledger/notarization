// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify an Object Proof
//!
//! An application can request a Proof of Inclusion using only an object ID. The
//! builder fetches its latest version at proof construction time, discovers the
//! transaction that produced it, and packages both as one proof.
//!
//! The discovered transaction is evidence supporting the object claim. It is not
//! an explicit transaction target unless the caller also invokes `transaction`.
//!
//! Verification authenticates the checkpoint committee from a trusted genesis
//! blob, independently of the node that supplied the proof evidence.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Request a proof using only an object ID.
/// 2. Let the builder resolve the latest object version and its transaction.
/// 3. Distinguish supporting transaction evidence from an explicit target.
/// 4. Authenticate committee history from genesis and verify the object claim.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify an Object Proof ===\n");

    let context = prepare_poi_example().await?;
    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    println!("Committee resolution: genesis anchored");

    let object_id = context.create_notarization("PoI object-proof example").await?.object_id;
    let client = &context.poi_client;

    println!("Network:       {}", context.network_alias);
    println!("Object target: {object_id}\n");

    // No transaction digest is supplied. The builder fetches the object first,
    // reads its previous transaction, and constructs evidence for that execution.
    let proof = client
        .proof()
        .object(object_id)
        .build()
        .await
        .context("failed to construct the object proof")?;

    ensure!(
        proof.targets().transaction.is_none(),
        "the discovered transaction must not become an explicit transaction target"
    );
    ensure!(
        proof.targets().objects.len() == 1,
        "the proof must contain one object target"
    );
    ensure!(
        proof.targets().objects[0].as_inner().object_ref().object_id == object_id,
        "the proof must contain the requested object"
    );
    ensure!(
        proof.targets().events.is_empty(),
        "the object-only proof must not contain event targets"
    );

    println!("Proof constructed:");
    println!(
        "  resolved transaction: {}",
        proof.transaction_proof().transaction.digest()
    );
    println!("  object version:       {}", proof.targets().objects[0].version());
    println!("  checkpoint epoch:     {}", proof.checkpoint_summary().epoch());
    println!(
        "  checkpoint number:    {}\n",
        proof.checkpoint_summary().sequence_number
    );

    let verifier = client.verifier(resolution);
    let verified = verifier
        .verify(&proof)
        .await
        .context("object proof verification failed")?;

    println!("Object proof verified successfully.");
    println!(
        "  authenticated object: {:?}",
        verified.objects()[0].as_inner().object_ref()
    );

    Ok(())
}
