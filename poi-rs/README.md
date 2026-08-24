# IOTA Proof of Inclusion Rust Package

The Proof of Inclusion Rust package provides proof data types and offline verification for inclusion claims in the IOTA
Notarization Toolkit.

Use Proof of Inclusion when a verifier needs cryptographic evidence that a transaction, event, or object state is tied to
a certified IOTA checkpoint. `ProofBuilder` fetches the proof material, while `ProofVerifier` verifies that material
locally without trusting the source that supplied it.

## Proof Construction

`PoiClient` provides explicit constructors for the public IOTA networks. The client does not select a default network, so
the calling application always chooses where it fetches proof material.

```rust,no_run
use iota_sdk_types::TransactionDigest;
use poi_rs::PoiClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let transaction_digest: TransactionDigest = todo!();
let client = PoiClient::mainnet()?;
let proof = client
    .proof()
    .transaction(transaction_digest)
    .build()
    .await?;
# Ok(())
# }
```

Use `PoiClient::testnet()` or `PoiClient::devnet()` for the other public networks. Applications can pass a custom `Source`
to `PoiClient::new(source)` when they use a private node, archive, fixture, or local test cluster. `ProofBuilder` remains
available directly for lower-level use.

A builder can stack multiple object and event targets by calling `object()` and `event()` repeatedly or by using the
`objects()` and `events()` batch methods. Every target must belong to the same transaction. The builder ignores exact
duplicates and reuses one transaction and one checkpoint for the complete target set.

`Source` is the transport boundary: it fetches decoded transaction, object, checkpoint, chain, and committee evidence.
`ProofBuilder` owns target resolution, consistency checks, and proof construction, while `CommitteeResolver` owns
committee authentication and caching, so custom sources do not reimplement either workflow.

Network selection configures only the proof source. It does not make the returned proof trusted or select an authoritative
committee for verification.

The default `native-grpc` feature implements `Source` directly for the SDK `GrpcClient` and provides the public-network
constructors. WASM packages can disable default features and supply a JavaScript-backed `Source` without compiling native
gRPC.

## Proof Model

A `Proof` contains three layers of evidence:

- `ProofTargets` recording the transaction, objects, and events explicitly selected by the caller.
- A `CertifiedCheckpointSummary` and its `CheckpointContents` linking the transaction to a committee-certified
  checkpoint.
- A required `TransactionProof` containing the transaction, its effects, and event data when event targets are present.

Object targets contain their exact object values; verification derives each object reference and finds it in the
transaction effects. Event targets contain `EventID` values, while the transaction proof carries the complete event list
needed to verify the effects' event digest. A transaction target is present only when the caller explicitly requested the
transaction itself, although transaction evidence supports every proof.

## Verification

For the common source-backed workflow, create a verifier from the same `PoiClient`. The verifier resolves the committee
required by the proof and then performs offline proof verification:

```rust,no_run
use std::fs::File;

use poi_rs::{CommitteeResolution, PoiClient, Proof};

# async fn example(proof: &Proof) -> Result<(), Box<dyn std::error::Error>> {
let client = PoiClient::testnet()?;
let resolution = CommitteeResolution::from_genesis(File::open("genesis.blob")?)?;
let verifier = client.verifier(resolution);

verifier.verify(proof).await?;
# Ok(())
# }
```

`CommitteeResolution::TrustedNode` is available when the connected node is explicitly inside the caller's trust
boundary. `CommitteeResolution::from_genesis()` loads an anchor committee from a trusted BCS-encoded genesis blob,
while `CommitteeResolution::anchored()` accepts an already extracted trusted committee. Use
`CommitteeResolution::anchored_with_cache()` or `CommitteeResolution::from_genesis_with_cache()` to supply a cache
that contains committees authenticated for the same network.
Retain the verifier when checking multiple proofs so its authenticated committee cache is reused.

`ProofVerifier` remains the offline verification entry point for callers that already possess the authoritative
committee. It verifies only the proof material passed by the caller.

Verification checks:

- the checkpoint summary is certified by the supplied committee
- the checkpoint contents match the certified checkpoint summary
- the transaction digest matches the transaction effects
- the transaction effects are included in the checkpoint contents
- an explicitly requested transaction matches the packaged transaction
- requested object targets derive references present in the transaction effects
- event data, when required, matches the event digest recorded in the effects
- requested event targets belong to the transaction and select events in the authenticated event list

## Trust Boundaries

`ProofVerifier` is intentionally offline. It does not make RPC calls and does not decide which committee is authoritative.
`CommitteeResolver::verify()` composes committee resolution with offline verification for source-backed workflows.
`CommitteeResolver::resolve()` remains available when callers need the authenticated committee itself.

The verifier treats all proof payloads as untrusted until verification succeeds. After verification succeeds, callers can
trust the authenticated target claims relative to the supplied committee.

## Main Types

- `Proof`: Versioned Proof of Inclusion envelope.
- `ProofV1`: Version 1 checkpoint and transaction evidence carried by `Proof::ProofV1`.
- `TransactionProof`: Transaction, effects, and optional event evidence used to prove inclusion.
- `ProofTargets`: Transaction, object, and event claims explicitly selected by the caller.
- `PoiClient`: Source-backed entry point for proof construction and committee-aware verification.
- `CommitteeResolution`: Trusted-node or anchored committee-resolution configuration, including the committee cache.
- `ProofBuilder`: Network-aware or custom-source proof construction.
- `Source`: Ledger-read boundary for gRPC nodes, JavaScript clients, archives, fixtures, and other evidence sources.
- `SourceTransaction` and `SourceCheckpoint`: Transport-independent decoded evidence returned by a `Source`.
- `CommitteeResolver`: Committee resolution and source-backed proof verification configured by `CommitteeResolution`.
- `ProofVerifier`: Offline verifier for `Proof` values.
- `SourceError`: Transport and response failures from a ledger source.
- `ProofBuilderError`, `CommitteeResolutionError`, `ProofVerificationError`, `VerifyError`, and `SerializationError`:
  Operation-specific errors.
