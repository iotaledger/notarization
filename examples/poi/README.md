# IOTA Proof of Inclusion Examples

These Rust examples create fresh ledger activity and use it to construct and verify portable IOTA Proof of Inclusion
proofs. Each example reads the active IOTA CLI environment and wallet, creates a locked `Notarization` object, and uses
the resulting transaction, object, or event as its proof target.

Proof construction and verification have separate trust responsibilities. The configured gRPC endpoint supplies
untrusted ledger evidence. Genesis-anchored verification authenticates that evidence from an independently trusted
genesis blob, while trusted-node verification places the connected node inside the verifier's trust boundary.

## Prerequisites

Configure an IOTA CLI environment and active wallet address before running an example. The examples use the CLI
keystore through `KeytoolSigner`; they do not require an IOTA private-key environment variable.

For localnet, start the installed launcher with JSON-RPC, faucet, and gRPC services:

```bash
iota-localnet start --with-faucet --with-grpc
```

The standard localnet gRPC endpoint is `http://127.0.0.1:50051`.

The shared setup performs the following work:

- It uses the active CLI environment's JSON-RPC endpoint for Single Notarization transactions.
- It uses the environment's configured gRPC endpoint, a known public-network endpoint, or `NETWORK_GRPC_URL`.
- It uses `IOTA_NOTARIZATION_PKG_ID` when provided.
- On non-mainnet networks, it requests faucet funds when required and publishes the Single Notarization Move Package
  when no package ID is configured or cached.
- On mainnet, it never requests faucet funds or publishes the package automatically.

| Environment variable       | Required when                                                                                       |
| :------------------------- | :-------------------------------------------------------------------------------------------------- |
| `IOTA_NOTARIZATION_PKG_ID` | Always on mainnet; optional on other networks.                                                      |
| `NETWORK_GRPC_URL`         | The active CLI environment has no gRPC URL and does not resolve to a known public network/localnet. |
| `IOTA_GENESIS_PATH`        | A genesis-anchored example runs against localnet or a custom network.                               |

> [!IMPORTANT]
> Mainnet examples submit paid transactions from the active CLI wallet. Set `IOTA_NOTARIZATION_PKG_ID` to an existing
> Single Notarization Move Package and fund the active wallet before running them.

Examples 01, 02, 03, and the advanced file-cache example use genesis-anchored verification. Mainnet, testnet, and
devnet genesis blobs download automatically and remain cached in the IOTA configuration directory. Local and custom
networks require `IOTA_GENESIS_PATH` because the verifier cannot infer a trusted genesis source for them.

## Running an Example

Run an example from the repository root:

```bash
cargo run --release -p poi-examples --example 01_create_and_verify_transaction_proof
```

For localnet genesis-anchored examples, provide the genesis blob created by that local network:

```bash
export IOTA_GENESIS_PATH=/path/to/localnet/genesis.blob
cargo run --release -p poi-examples --example 01_create_and_verify_transaction_proof
```

The focused runner executes every example:

```bash
./examples/poi/run.sh
```

The complete runner creates seven locked `Notarization` objects because the verifier-reuse example creates two
transactions. A non-mainnet run may also publish the Single Notarization Move Package once.

## Examples

| Name                                                                                    | Information                                                                                                   |
| :-------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------ |
| [01_create_and_verify_transaction_proof](./01_create_and_verify_transaction_proof.rs)   | Creates a transaction proof, serializes it as JSON, and verifies it from a trusted network genesis blob.      |
| [02_create_and_verify_multi_target_proof](./02_create_and_verify_multi_target_proof.rs) | Combines transaction, changed-object, and emitted-event targets in one proof.                                 |
| [03_reuse_verifier_for_multiple_proofs](./03_reuse_verifier_for_multiple_proofs.rs)     | Reuses one genesis-anchored verifier across proofs for two fresh transactions.                                |
| [04_create_and_verify_object_proof](./04_create_and_verify_object_proof.rs)             | Starts from a fresh object ID and lets the builder discover the transaction that created its latest version. |
| [05_create_and_verify_event_proof](./05_create_and_verify_event_proof.rs)               | Starts from a fresh event ID without declaring a separate transaction target.                                |
| [advanced_01_file_based_committee_cache](./advanced/01_file_based_committee_cache.rs)   | Persists authenticated committees in a cache scoped to the active network.                                   |

## Example Workflow

The shared setup creates the on-chain evidence before each example constructs a proof. The transaction example targets
the creation transaction. The multi-target example targets that transaction, its created `Notarization` object, and
its `LockedNotarizationCreated` event in one proof.

The verifier-reuse example creates two transactions and retains one verifier across both proofs. The second
verification reuses any committee history authenticated during the first verification. The object and event examples
use trusted-node committee resolution so they can focus on target-driven discovery without performing a genesis walk.

## Trust Boundaries

- Selecting a network configures the source of proof material; it does not make the proof trusted.
- The `chain` value carried by a proof is informational and must not act as a trust anchor.
- Treat the complete proof payload as untrusted until verification succeeds.
- Obtain `IOTA_GENESIS_PATH` independently from the party supplying the proof.
- Ensure the genesis blob belongs to the same network as the proof.
- Scope persistent committee caches to one network and genesis anchor.
- Use `CommitteeResolution::TrustedNode` only when the connected node is inside the verifier's trust boundary.
