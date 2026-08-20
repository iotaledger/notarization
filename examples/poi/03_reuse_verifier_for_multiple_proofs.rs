// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Reuse a Verifier for Multiple Proofs
//!
//! Genesis-anchored verification authenticates every committee transition from
//! epoch 0 to the proof's checkpoint epoch. A verifier retains each authenticated
//! committee in its cache, so applications should reuse it across proofs from the
//! same network instead of creating a fresh verifier for every proof.
//!
//! This example constructs proofs for two different mainnet transactions from
//! epoch 469. The first verification performs the committee walk. The second
//! verification reuses the already authenticated epoch-469 committee.

use std::time::Instant;

use anyhow::{Context, Result, ensure};
use iota_sdk_types::TransactionDigest;
use poi_rs::{CommitteeResolution, PoiClient};

mod utils;

use utils::load_mainnet_genesis;

const MAINNET_TRANSACTION_DIGEST: &str = "86EvhdjqBb6Rt5pB8sKjTnE7MrzpNLuWTH3SuELBjDvu";
const SECOND_MAINNET_TRANSACTION_DIGEST: &str = "G8hfzqq9tCSEHF4cq9NMCZyKemuShmJoqfDDoG4K3C6z";

/// Demonstrates how to:
/// 1. Construct independent proofs for two mainnet transactions.
/// 2. Create one genesis-anchored verifier.
/// 3. Verify the first proof and populate the authenticated committee cache.
/// 4. Reuse the same verifier for the second proof.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Reuse a Verifier for Multiple Proofs ===\n");

    let first_transaction = MAINNET_TRANSACTION_DIGEST
        .parse::<TransactionDigest>()
        .context("the first example transaction digest must be valid")?;
    let second_transaction = SECOND_MAINNET_TRANSACTION_DIGEST
        .parse::<TransactionDigest>()
        .context("the second example transaction digest must be valid")?;
    let client = PoiClient::mainnet().context("failed to configure the public mainnet gRPC endpoint")?;

    // Each builder creates an independent portable proof. Both transactions are
    // from epoch 469, but they are included in different checkpoints.
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

    ensure!(
        first_proof.checkpoint_summary.epoch() == second_proof.checkpoint_summary.epoch(),
        "both example proofs must belong to the same epoch"
    );

    println!("First transaction:  {first_transaction}");
    println!("First checkpoint:   {}", first_proof.checkpoint_summary.sequence_number);
    println!("Second transaction: {second_transaction}");
    println!(
        "Second checkpoint:  {}",
        second_proof.checkpoint_summary.sequence_number
    );
    println!("Checkpoint epoch:   {}\n", first_proof.checkpoint_summary.epoch());

    let genesis = load_mainnet_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;

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
    println!("The second verification reused the authenticated epoch-469 committee.");

    Ok(())
}
