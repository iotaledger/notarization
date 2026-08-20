# IOTA Proof of Inclusion Examples

The following code examples demonstrate how to construct and verify portable IOTA Proof of Inclusion proofs.

Proof construction and proof verification have separate trust responsibilities. A ledger endpoint supplies the
transaction, checkpoint, and committee-transition evidence used by a proof. That evidence remains untrusted until a
verifier authenticates the relevant checkpoint committee and verifies the proof locally.

## Prerequisites

The first example uses IOTA mainnet and requires:

- network access to the public IOTA gRPC endpoint.
- network access to download the official mainnet `genesis.blob` on the first run.

The example uses the last transaction in mainnet epoch 469, checkpoint 179,638,743. The transaction digest is fixed so
the example can be run without configuration. The genesis blob is downloaded from the official IOTA distribution URL
and cached in the IOTA configuration directory. Mainnet nodes prune historical transaction data, so this digest must be
replaced with a transaction from a newer completed epoch when the public gRPC endpoint no longer serves it.

## Running the Example

Run the example from the repository root:

```bash
cargo run --release -p poi-examples --example 01_create_and_verify_transaction_proof
```

Alternatively, use the focused runner:

```bash
./examples/poi/run.sh
```

> [!NOTE]
> Genesis-based committee resolution authenticates every committee transition from epoch 0 to epoch 469. The first
> verification can therefore take a long time. Retain and reuse the verifier when verifying multiple proofs so its
> in-memory committee cache can be reused.

## Examples

| Name                                                                                                     | Information                                                                                                 |
| :------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------- |
| [01_create_and_verify_transaction_proof](./01_create_and_verify_transaction_proof.rs)                    | Constructs a transaction proof, serializes it as JSON, and verifies it from a trusted network genesis blob. |
| [02_create_and_verify_multi_target_proof](./02_create_and_verify_multi_target_proof.rs)                  | Combines transaction, changed-object, and emitted-event targets in one proof and verifies every claim.      |
| [03_reuse_verifier_for_multiple_proofs](./03_reuse_verifier_for_multiple_proofs.rs)                      | Reuses one genesis-anchored verifier so authenticated committee history is not fetched and checked twice.   |
| [04_create_and_verify_object_proof](./04_create_and_verify_object_proof.rs)                              | Starts from an object ID and lets the builder discover its version and the transaction that changed it.     |
| [05_create_and_verify_event_proof](./05_create_and_verify_event_proof.rs)                                | Starts from an event ID and proves the selected event without declaring a separate transaction target.      |

Advanced integration examples, including persistent committee caching, are documented in
[advanced/README.md](./advanced/README.md).

## Example Workflow

The first example follows the complete transaction-proof lifecycle:

1. Connect to the public IOTA mainnet proof source.
2. Construct a proof for an existing transaction.
3. Serialize and deserialize the proof to model transfer between parties.
4. Download and cache the official mainnet genesis blob.
5. Authenticate committee transitions up to the checkpoint epoch.
6. Verify the proof locally.

The second example builds on that workflow by selecting three related targets from the same transaction. The builder
resolves the object's exact version from the transaction effects and includes the complete event data needed to prove
the selected event. A single verification then authenticates all declared targets.

The third example constructs proofs for two transactions from epoch 469 and verifies both with one verifier. The first
verification walks committee history from genesis and fills the verifier's authenticated in-memory cache. The second
verification resolves the same epoch directly from that cache. Applications that verify multiple proofs should retain
their verifier for this reason.

The fourth example demonstrates target-driven discovery. It supplies only an object ID, allowing the builder to fetch
the latest object version and discover the transaction that produced it. The fifth example supplies only an event ID;
because that ID contains its transaction digest, the builder can resolve the complete event evidence directly. In both
cases the resolved transaction supports the requested claim without becoming an explicit transaction target. These two
focused examples use trusted-node resolution to avoid the genesis committee walk.

## Trust Boundaries

- Selecting a network configures the source of proof material; it does not make the proof trusted.
- The `chain` value carried by a proof is informational and must not be used as a trust anchor.
- Treat the complete proof payload as untrusted until verification succeeds.
- The trusted genesis blob must belong to the same network as the proof and be obtained independently from the party
  supplying the proof.
- Retain a verifier when checking multiple proofs so its authenticated committee cache can be reused.
- Examples 04 and 05 use `CommitteeResolution::TrustedNode`; this is appropriate only when the connected node is inside
  the verifier's trust boundary because committee lineage is not authenticated from genesis.
