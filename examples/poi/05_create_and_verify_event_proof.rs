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
//! This focused example uses trusted-node committee resolution to avoid an epoch
//! walk. Use this mode only when the connected node is inside the verifier's
//! trust boundary.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Identify an event by transaction digest and sequence number.
/// 2. Construct a proof without adding an explicit transaction target.
/// 3. Inspect the event target and its supporting event evidence.
/// 4. Resolve the committee from a trusted node and verify the event claim.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify an Event Proof ===\n");

    let context = prepare_poi_example().await?;
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

    // Trusted-node resolution accepts the committee reported by the connected
    // node. It avoids the genesis walk but makes that node part of the trust boundary.
    let verifier = client.verifier(CommitteeResolution::TrustedNode);
    verifier
        .verify(&proof)
        .await
        .context("event proof verification failed")?;

    println!("Event proof verified successfully.");
    println!("The selected event was emitted by a transaction included in the verified checkpoint.");

    Ok(())
}
