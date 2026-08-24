// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify a Transaction Proof
//!
//! This example creates an IOTA transaction, constructs a Proof of Inclusion for it,
//! serializes the proof for transfer, and verifies it from a trusted genesis blob.
//!
//! ## Actors
//!
//! - **Ledger source**: Supplies untrusted transaction, checkpoint, and committee-transition evidence.
//! - **Prover application**: Selects the transaction and constructs a portable proof from the ledger evidence.
//! - **Verifier application**: Starts from an independently trusted genesis blob, authenticates the checkpoint
//!   committee, and verifies the proof locally.
//!
//! The example uses one [`poi_rs::PoiClient`] for both workflows, but the source endpoint and the genesis blob have
//! different security roles: the endpoint supplies evidence, while the genesis blob establishes trust.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::{CommitteeResolution, Proof};

/// Demonstrates how to:
/// 1. Configure clients from the active IOTA CLI environment and wallet.
/// 2. Establish committee trust from the active network's genesis blob.
/// 3. Create a transaction as fresh proof evidence.
/// 4. Construct its Proof of Inclusion.
/// 5. Serialize and deserialize the proof as portable JSON.
/// 6. Verify the received proof locally.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify a Transaction Proof ===\n");

    // -------------------------------------------------------------------------
    // Step 1: Configure clients from the active IOTA CLI environment and wallet
    // -------------------------------------------------------------------------
    let context = prepare_poi_example().await?;

    // -------------------------------------------------------------------------
    // Step 2: Establish committee trust from the active network's genesis blob
    // -------------------------------------------------------------------------
    // The genesis blob must be obtained independently from an authoritative
    // source and must belong to the same network as the proof.
    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    let verifier = context.poi_client.verifier(resolution);

    // -------------------------------------------------------------------------
    // Step 3: Create a transaction as fresh proof evidence
    // -------------------------------------------------------------------------
    let transaction_digest = context
        .create_notarization("PoI transaction-proof example")
        .await?
        .transaction_digest;
    let client = &context.poi_client;

    // Selecting a network controls where proof material is fetched. It does not
    // make that material trusted and does not select an authoritative committee.
    println!("Network:            {}", context.network_alias);
    println!("Transaction target: {transaction_digest}\n");

    // -------------------------------------------------------------------------
    // Step 4: Construct its Proof of Inclusion
    // -------------------------------------------------------------------------
    // The builder fetches the transaction, its effects, and the certified
    // checkpoint evidence required to prove inclusion.
    let proof = client
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .context("failed to construct the transaction proof")?;

    ensure!(
        proof.targets().transaction == Some(transaction_digest),
        "the constructed proof must contain the requested transaction target"
    );
    ensure!(
        proof.targets().objects.is_empty() && proof.targets().events.is_empty(),
        "this example should contain only a transaction target"
    );

    println!("Proof constructed:");
    println!("  format version:    {}", proof.version().value());
    println!("  reported chain:    {:?}", proof.chain);
    println!("  checkpoint epoch:  {}", proof.checkpoint_summary.epoch());
    println!("  checkpoint number: {}\n", proof.checkpoint_summary.sequence_number);

    // -------------------------------------------------------------------------
    // Step 5: Serialize and deserialize the proof as portable JSON
    // -------------------------------------------------------------------------
    // A verifier can receive this JSON through a file, API, message, or any
    // other transport. Proof payloads remain untrusted until verification succeeds.
    let proof_json = proof.to_json_vec().context("failed to serialize the proof as JSON")?;
    println!("Serialized proof size: {} bytes", proof_json.len());

    let received_proof = Proof::from_json_slice(&proof_json).context("failed to deserialize the received proof")?;
    ensure!(
        received_proof.targets().transaction == Some(transaction_digest),
        "the transaction target must survive the JSON round trip"
    );

    // -------------------------------------------------------------------------
    // Step 6: Verify the received proof locally
    // -------------------------------------------------------------------------
    // The resolver authenticates every committee transition from genesis to the
    // proof's epoch. Reuse the verifier when checking multiple proofs so its
    // in-memory committee cache can avoid repeating the walk.
    verifier
        .verify(&received_proof)
        .await
        .context("transaction proof verification failed")?;

    println!("\nTransaction proof verified successfully.");
    println!("The transaction is included in a checkpoint authenticated from the trusted genesis blob.");

    Ok(())
}
