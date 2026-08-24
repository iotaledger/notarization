// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Create and Verify an Object Proof
//!
//! An application can request a Proof of Inclusion using only an object ID. The
//! builder fetches the object's latest version, discovers the transaction that
//! produced that version, and packages the object and transaction evidence into
//! one proof.
//!
//! The discovered transaction is evidence supporting the object claim. It is not
//! an explicit transaction target unless the caller also invokes `transaction`.
//!
//! This focused example uses trusted-node committee resolution to avoid an epoch
//! walk. Use this mode only when the connected node is inside the verifier's
//! trust boundary.

use anyhow::{Context, Result, ensure};
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Request a proof using only an object ID.
/// 2. Let the builder resolve the latest object version and its transaction.
/// 3. Distinguish supporting transaction evidence from an explicit target.
/// 4. Resolve the committee from a trusted node and verify the object claim.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion: Create and Verify an Object Proof ===\n");

    let context = prepare_poi_example().await?;
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
        proof.transaction_proof.transaction.digest()
    );
    println!("  object version:       {}", proof.targets().objects[0].version());
    println!("  checkpoint epoch:     {}", proof.checkpoint_summary.epoch());
    println!("  checkpoint number:    {}\n", proof.checkpoint_summary.sequence_number);

    // Trusted-node resolution accepts the committee reported by the connected
    // node. It avoids the genesis walk but makes that node part of the trust boundary.
    let verifier = client.verifier(CommitteeResolution::TrustedNode);
    verifier
        .verify(&proof)
        .await
        .context("object proof verification failed")?;

    println!("Object proof verified successfully.");
    println!("The resolved object version was changed by a transaction included in the verified checkpoint.");

    Ok(())
}
