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
use iota_sdk_types::{ObjectId, TransactionDigest};
use iota_types::event::EventID;
use poi_rs::{CommitteeResolution, PoiClient, Proof};

mod utils;

use utils::load_mainnet_genesis;

const MAINNET_TRANSACTION_DIGEST: &str = "86EvhdjqBb6Rt5pB8sKjTnE7MrzpNLuWTH3SuELBjDvu";
const STAKED_IOTA_OBJECT_ID: &str = "0x001270619f0ff6c5fce1925838a132241c73b9756dae9d0dcfab71bb03549f73";
const STAKING_REQUEST_EVENT_SEQUENCE: u64 = 0;

/// Demonstrates how to:
/// 1. Select transaction, object, and event targets from one execution.
/// 2. Construct one proof containing all three claims.
/// 3. Inspect the targets resolved by the builder.
/// 4. Transfer the proof through its portable JSON representation.
/// 5. Authenticate the checkpoint committee from mainnet genesis.
/// 6. Verify every target in one operation.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify a Multi-Target Proof ===\n");

    let transaction_digest = MAINNET_TRANSACTION_DIGEST
        .parse::<TransactionDigest>()
        .context("the example transaction digest must be valid")?;
    let object_id = STAKED_IOTA_OBJECT_ID
        .parse::<ObjectId>()
        .context("the example object ID must be valid")?;
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: STAKING_REQUEST_EVENT_SEQUENCE,
    };

    let client = PoiClient::mainnet().context("failed to configure the public mainnet gRPC endpoint")?;

    println!("Network:            mainnet");
    println!("Transaction target: {transaction_digest}");
    println!("Object target:      {object_id}");
    println!("Event target:       {transaction_digest}:{STAKING_REQUEST_EVENT_SEQUENCE}\n");

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

    let genesis = load_mainnet_genesis().await?;
    let resolution = CommitteeResolution::from_genesis(genesis)
        .context("failed to load the committee from the trusted genesis blob")?;
    let verifier = client.verifier(resolution);

    // The proof is from epoch 469, so genesis-anchored resolution authenticates
    // every preceding committee transition. This may take a long time.
    verifier
        .verify(&received_proof)
        .await
        .context("multi-target proof verification failed")?;

    println!("\nMulti-target proof verified successfully.");
    println!("The transaction, changed object, and emitted event are authenticated by one proof.");

    Ok(())
}
