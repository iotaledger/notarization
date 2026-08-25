// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Reuse a Verifier for Multiple Proofs
//!
//! Genesis-anchored verification authenticates every committee transition from
//! epoch 0 to the proof's checkpoint epoch. A verifier retains each authenticated
//! committee in its cache, so applications should reuse it across proofs from the
//! same network instead of creating a fresh verifier for every proof.
//!
//! This example creates two transactions on the active network and constructs a
//! proof for each. The second verification reuses committee history already
//! authenticated while verifying the first proof.

use std::time::Instant;

use anyhow::{Context, Result};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Establish committee trust from the active network's genesis blob.
/// 2. Create and construct independent proofs for two transactions.
/// 3. Create one genesis-anchored verifier.
/// 4. Verify the first proof and populate the authenticated committee cache.
/// 5. Reuse the same verifier for the second proof.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Reuse a Verifier for Multiple Proofs ===\n");

    let context = prepare_poi_example().await?;
    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    let first_transaction = context
        .create_notarization("PoI verifier-reuse example, transaction 1")
        .await?
        .transaction_digest;
    let second_transaction = context
        .create_notarization("PoI verifier-reuse example, transaction 2")
        .await?
        .transaction_digest;
    let client = &context.poi_client;

    // Each builder creates an independent portable proof.
    let first_proof = client
        .proof()
        .transaction(first_transaction)
        .build()
        .await
        .context("failed to construct the first transaction proof")?;
    let second_proof = client
        .proof()
        .transaction(second_transaction)
        .build()
        .await
        .context("failed to construct the second transaction proof")?;

    println!("Network:            {}", context.network_alias);
    println!("First transaction:  {first_transaction}");
    println!(
        "First checkpoint:   {}",
        first_proof.checkpoint_summary().sequence_number
    );
    println!("First epoch:        {}", first_proof.checkpoint_summary().epoch());
    println!("Second transaction: {second_transaction}");
    println!(
        "Second checkpoint:  {}",
        second_proof.checkpoint_summary().sequence_number
    );
    println!("Second epoch:       {}\n", second_proof.checkpoint_summary().epoch());

    // Keep this verifier alive. Its default in-memory cache stores only
    // committees authenticated through the genesis-anchored epoch walk.
    let verifier = client.verifier(resolution);

    println!("Verifying the first proof; this performs the epoch walk...");
    let first_started = Instant::now();
    verifier
        .verify(&first_proof)
        .await
        .context("first transaction proof verification failed")?;
    let first_elapsed = first_started.elapsed();

    println!("Verifying the second proof with the same verifier...");
    let second_started = Instant::now();
    verifier
        .verify(&second_proof)
        .await
        .context("second transaction proof verification failed")?;
    let second_elapsed = second_started.elapsed();

    println!("\nBoth transaction proofs verified successfully.");
    println!("First verification:  {first_elapsed:?}");
    println!("Second verification: {second_elapsed:?}");
    println!("The second verification reused committee history authenticated during the first verification.");

    Ok(())
}
