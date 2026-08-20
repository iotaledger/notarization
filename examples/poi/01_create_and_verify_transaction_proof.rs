// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify a Transaction Proof
//!
//! This example constructs a Proof of Inclusion for an existing IOTA transaction,
//! serializes the proof for transfer, and verifies it from a trusted genesis blob.
//!
//! ## Actors
//!
//! - **Ledger source**: Supplies untrusted transaction, checkpoint, and committee-transition evidence.
//! - **Prover application**: Selects the transaction and constructs a portable proof from the ledger evidence.
//! - **Verifier application**: Starts from an independently trusted genesis blob, authenticates the checkpoint
//!   committee, and verifies the proof locally.
//!
//! The example uses one [`PoiClient`] for both workflows, but the source endpoint and the genesis blob have different
//! security roles: the endpoint supplies evidence, while the genesis blob establishes trust.

use anyhow::{Context, Result, ensure};
use iota_sdk_types::TransactionDigest;
use poi_rs::{CommitteeResolution, PoiClient, Proof};

mod utils;

use utils::load_mainnet_genesis;

const MAINNET_TRANSACTION_DIGEST: &str = "86EvhdjqBb6Rt5pB8sKjTnE7MrzpNLuWTH3SuELBjDvu";

/// Demonstrates how to:
/// 1. Connect to the IOTA mainnet proof source.
/// 2. Construct a Proof of Inclusion for a transaction.
/// 3. Serialize and deserialize the proof as portable JSON.
/// 4. Download and cache the trusted mainnet genesis blob.
/// 5. Authenticate the checkpoint committee from genesis.
/// 6. Verify the received proof locally.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify a Transaction Proof ===\n");

    let transaction_digest = MAINNET_TRANSACTION_DIGEST
        .parse::<TransactionDigest>()
        .context("the example transaction digest must be valid")?;

    // -------------------------------------------------------------------------
    // Step 1: Connect to the mainnet proof source
    // -------------------------------------------------------------------------
    // Selecting a network controls where proof material is fetched. It does not
    // make that material trusted and does not select an authoritative committee.
    let client = PoiClient::mainnet().context("failed to configure the public mainnet gRPC endpoint")?;

    println!("Network:            mainnet");
    println!("Transaction target: {transaction_digest}\n");

    // -------------------------------------------------------------------------
    // Step 2: Construct the transaction proof
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
    // Step 3: Serialize the proof for transfer
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
    // Step 4: Establish trust from the network genesis blob
    // -------------------------------------------------------------------------
    // The genesis blob must be obtained independently from an authoritative
    // source and must belong to the same network as the proof.
    let genesis = load_mainnet_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    let verifier = client.verifier(resolution);

    // -------------------------------------------------------------------------
    // Step 5: Authenticate the committee and verify the proof locally
    // -------------------------------------------------------------------------
    // This transaction is from epoch 469. The resolver authenticates every
    // committee transition from genesis to that epoch, so this first run may
    // take a long time. Reuse the verifier when checking multiple proofs so its
    // in-memory committee cache can avoid repeating the walk.
    verifier
        .verify(&received_proof)
        .await
        .context("transaction proof verification failed")?;

    println!("\nTransaction proof verified successfully.");
    println!("The transaction is included in a checkpoint authenticated from the trusted genesis blob.");

    Ok(())
}
