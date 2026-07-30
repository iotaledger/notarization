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

- A `CertifiedCheckpointSummary` signed by the committee for the checkpoint epoch.
- A `TransactionProof` containing the checkpoint contents, transaction, effects, and optional events.
- `ProofTargets` describing the object, event, or committee claims the caller wants to authenticate.

The transaction proof is required. A Proof of Inclusion proves inclusion in a certified checkpoint, so the proof envelope
must carry the transaction evidence that links the target claim to the checkpoint contents.

## Verification

For the common source-backed workflow, create a verifier from the same `PoiClient`. The verifier resolves the committee
required by the proof and then performs offline proof verification:

```rust,no_run
use std::fs::File;

use poi_rs::{PoiClient, Proof};

# async fn example(proof: &Proof) -> Result<(), Box<dyn std::error::Error>> {
let client = PoiClient::testnet()?;
let verifier = client.anchored_at_genesis(File::open("genesis.blob")?)?;

verifier.verify(proof).await?;
# Ok(())
# }
```

`PoiClient::trusted_node()` is available when the connected node is explicitly inside the caller's trust boundary.
`PoiClient::anchored_at_genesis()` loads the anchor committee from a trusted BCS-encoded genesis blob.
`PoiClient::anchored_at()` accepts an already extracted trusted committee instead.
Retain the verifier when checking multiple proofs so its authenticated committee cache is reused.

`ProofVerifier` remains the offline verification entry point for callers that already possess the authoritative
committee. It verifies only the proof material passed by the caller.

Verification checks:

- the proof format version is supported
- the checkpoint summary is certified by the supplied committee
- the checkpoint contents match the certified checkpoint summary
- the transaction digest matches the transaction effects
- the transaction effects are included in the checkpoint contents
- packaged events match the event digest recorded in the effects
- requested event targets belong to the transaction and match the packaged event contents
- requested object targets match their object references and appear in the transaction effects
- requested committee targets match the next committee recorded in an end-of-epoch checkpoint

## Trust Boundaries

`ProofVerifier` is intentionally offline. It does not make RPC calls and does not decide which committee is authoritative.
`CommitteeResolver::verify()` composes committee resolution with offline verification for source-backed workflows.
`CommitteeResolver::resolve()` remains available when callers need the authenticated committee itself.

The verifier treats all proof payloads as untrusted until verification succeeds. After verification succeeds, callers can
trust the authenticated target claims relative to the supplied committee.

## Main Types

- `Proof`: Versioned Proof of Inclusion envelope.
- `ProofVersion`: Proof format version used for compatibility checks.
- `TransactionProof`: Transaction, effects, events, and checkpoint contents used to prove inclusion.
- `ProofTargets`: Object, event, and committee claims to authenticate.
- `ProofTarget`: Transaction, object, or event requested from a `ProofBuilder`.
- `PoiClient`: Source-backed entry point for proof construction and trusted-node or anchored verification.
- `ProofBuilder`: Network-aware or custom-source proof construction.
- `Source`: Ledger-read boundary for gRPC nodes, JavaScript clients, archives, fixtures, and other evidence sources.
- `SourceTransaction` and `SourceCheckpoint`: Transport-independent decoded evidence returned by a `Source`.
- `CommitteeResolver`: Trusted-node or anchored committee resolution and source-backed proof verification.
- `ProofVerifier`: Offline verifier for `Proof` values.
- `SourceError`: Transport and response failures from a ledger source.
- `ProofBuilderError`, `CommitteeResolutionError`, `ProofVerificationError`, `VerifyError`, `SerializationError`, and
  `VersionError`: Operation-specific errors.
