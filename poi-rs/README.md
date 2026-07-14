# IOTA Proof of Inclusion Rust Package

The Proof of Inclusion Rust package provides proof data types and offline verification for inclusion claims in the IOTA
Notarization Toolkit.

Use Proof of Inclusion when a verifier needs cryptographic evidence that a transaction, event, or object state is tied to
a certified IOTA checkpoint. `ProofBuilder` fetches the proof material, while `ProofVerifier` verifies that material
locally without trusting the source that supplied it.

## Proof Construction

`ProofBuilder` provides explicit constructors for the public IOTA networks. The builder does not select a default network,
so the calling application always chooses where it fetches proof material.

```rust,no_run
use iota_types::digests::TransactionDigest;
use poi_rs::ProofBuilder;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let transaction_digest: TransactionDigest = todo!();
let proof = ProofBuilder::mainnet()?
    .transaction(transaction_digest)
    .build()
    .await?;
# Ok(())
# }
```

Use `ProofBuilder::testnet()` or `ProofBuilder::devnet()` for the other public networks. Applications can pass a custom
`Source` to `ProofBuilder::new(source)` when they use a private node, archive, fixture, or local test cluster.

Network selection configures only the proof source. It does not make the returned proof trusted or select an authoritative
committee for verification.

## Proof Model

A `Proof` contains three layers of evidence:

- A `CertifiedCheckpointSummary` signed by the committee for the checkpoint epoch.
- A `TransactionProof` containing the checkpoint contents, transaction, effects, and optional events.
- `ProofTargets` describing the object, event, or committee claims the caller wants to authenticate.

The transaction proof is required. A Proof of Inclusion proves inclusion in a certified checkpoint, so the proof envelope
must carry the transaction evidence that links the target claim to the checkpoint contents.

## Verification

`ProofVerifier` is the public verification entry point. It receives the authoritative committee for the proof checkpoint
and verifies only the proof material passed by the caller.

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
Callers must provide the committee that should certify the checkpoint. `CommitteeResolver` can resolve committee history
before the caller invokes the verifier.

The verifier treats all proof payloads as untrusted until verification succeeds. After verification succeeds, callers can
trust the authenticated target claims relative to the supplied committee.

## Main Types

- `Proof`: Versioned Proof of Inclusion envelope.
- `ProofVersion`: Proof format version used for compatibility checks.
- `TransactionProof`: Transaction, effects, events, and checkpoint contents used to prove inclusion.
- `ProofTargets`: Object, event, and committee claims to authenticate.
- `ProofBuilder`: Network-aware or custom-source proof construction.
- `Source`: Extensible boundary for gRPC nodes, archives, fixtures, and other proof sources.
- `CommitteeResolver`: Trusted-node or anchored committee resolution.
- `ProofVerifier`: Offline verifier for `Proof` values.
- `VerifyError`, `SourceError`, `SerializationError`, and `VersionError`: Operation-specific errors.
