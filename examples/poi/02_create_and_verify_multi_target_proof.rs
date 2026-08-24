// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify a Multi-Target Proof
//!
//! A single Proof of Inclusion can authenticate several claims about the same
//! transaction. This example proves the transaction itself, one object changed
//! by the transaction, and one event emitted by the transaction.
//!
//! Combining related targets avoids duplicating the transaction and checkpoint
//! evidence across separate proofs. Every object and event target must belong to
//! the selected transaction; the builder rejects targets from another transaction.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::{CommitteeResolution, Proof};

/// Demonstrates how to:
/// 1. Create transaction, object, and event targets in one execution.
/// 2. Construct one proof containing all three claims.
/// 3. Inspect the targets resolved by the builder.
/// 4. Transfer the proof through its portable JSON representation.
/// 5. Authenticate the checkpoint committee from the active network's genesis.
/// 6. Verify every target in one operation.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify a Multi-Target Proof ===\n");

    let context = prepare_poi_example().await?;
    let targets = context.create_notarization("PoI multi-target example").await?;
    let transaction_digest = targets.transaction_digest;
    let object_id = targets.object_id;
    let event_id = targets.event_id;
    let client = &context.poi_client;

    println!("Network:            {}", context.network_alias);
    println!("Transaction target: {transaction_digest}");
    println!("Object target:      {object_id}");
    println!("Event target:       {transaction_digest}:{}\n", event_id.event_seq);

    // The explicit transaction and event identify one execution. The object ID
    // is resolved at the exact version recorded in that transaction's effects.
    let proof = client
        .proof()
        .transaction(transaction_digest)
        .object(object_id)
        .event(event_id)
        .build()
        .await
        .context("failed to construct the multi-target proof")?;

    ensure!(
        proof.targets().transaction == Some(transaction_digest),
        "the proof must contain the requested transaction target"
    );
    ensure!(
        proof.targets().objects.len() == 1,
        "the proof must contain one object target"
    );
    ensure!(
        proof.targets().objects[0].as_inner().object_ref().object_id == object_id,
        "the proof must contain the requested object target"
    );
    ensure!(
        proof.targets().events == [event_id],
        "the proof must contain the requested event target"
    );
    ensure!(
        proof.transaction_proof.events.is_some(),
        "event evidence must be present when an event is targeted"
    );

    println!("Proof constructed:");
    println!("  checkpoint epoch:  {}", proof.checkpoint_summary.epoch());
    println!("  checkpoint number: {}", proof.checkpoint_summary.sequence_number);
    println!("  object targets:    {}", proof.targets().objects.len());
    println!("  event targets:     {}\n", proof.targets().events.len());

    // A verifier receives the complete proof as untrusted input. JSON is used
    // here to model transfer across a file, API, message, or process boundary.
    let proof_json = proof.to_json_vec().context("failed to serialize the proof as JSON")?;
    println!("Serialized proof size: {} bytes", proof_json.len());
    let received_proof = Proof::from_json_slice(&proof_json).context("failed to deserialize the received proof")?;

    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    let verifier = client.verifier(resolution);

    // Genesis-anchored resolution authenticates every preceding committee
    // transition. This may take a long time on an established network.
    verifier
        .verify(&received_proof)
        .await
        .context("multi-target proof verification failed")?;

    println!("\nMulti-target proof verified successfully.");
    println!("The transaction, changed object, and emitted event are authenticated by one proof.");

    Ok(())
}
