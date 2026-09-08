// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify an Event Proof
//!
//! An event ID contains the digest of its emitting transaction and the event's
//! sequence number. That makes the event ID sufficient to select an execution:
//! the caller does not need to add a separate transaction target.
//!
//! Event proofs carry the transaction's complete event list because the event
//! digest in the transaction effects commits to that complete list. Verification
//! then checks that the selected sequence exists in the authenticated events.
//!
//! Verification authenticates the checkpoint committee from a trusted genesis
//! blob, independently of the node that supplied the proof evidence.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Identify an event by transaction digest and sequence number.
/// 2. Construct a proof without adding an explicit transaction target.
/// 3. Inspect the event target and its supporting event evidence.
/// 4. Authenticate committee history from genesis and verify the event target.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify an Event Proof ===\n");

    let context = prepare_poi_example().await?;
    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    println!("Committee resolution: genesis anchored");

    let event_id = context.create_notarization("PoI event-proof example").await?.event_id;
    let client = &context.poi_client;

    println!("Network:      {}", context.network_alias);
    println!("Event target: {}:{}\n", event_id.tx_digest, event_id.event_seq);

    // The event ID already identifies the emitting transaction, so the builder
    // can fetch the required transaction, effects, checkpoint, and event data.
    let proof = client
        .proof()
        .event(event_id)
        .build()
        .await
        .context("failed to construct the event proof")?;

    ensure!(
        proof.targets().transaction.is_none(),
        "the emitting transaction must not become an explicit transaction target"
    );
    ensure!(
        proof.targets().objects.is_empty(),
        "the event-only proof must not contain object targets"
    );
    ensure!(
        proof.targets().events == [event_id],
        "the proof must contain the requested event target"
    );
    ensure!(
        proof.transaction_proof().events.is_some(),
        "the proof must contain the event data committed to by the transaction effects"
    );

    println!("Proof constructed:");
    println!(
        "  resolved transaction: {}",
        proof.transaction_proof().transaction.digest()
    );
    println!("  checkpoint epoch:     {}", proof.checkpoint_summary().epoch());
    println!(
        "  checkpoint number:    {}\n",
        proof.checkpoint_summary().sequence_number
    );

    let verifier = client.verifier(resolution);
    let verified = verifier
        .verify(&proof)
        .await
        .context("event proof verification failed")?;

    println!("Event proof verified successfully.");
    for (event_id, event) in verified.events() {
        println!(
            "  authenticated event: {event_id:?} ({} BCS bytes)",
            event.contents.len()
        );
    }

    Ok(())
}
