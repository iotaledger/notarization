# IOTA Proof of Inclusion WASM Examples

The examples in this folder demonstrate how to construct and verify portable
IOTA Proof of Inclusion proofs with the `@iota/poi-wasm` Package.

Proof construction and verification have separate trust responsibilities. The
mainnet endpoint supplies transaction, checkpoint, and committee-transition
evidence. That evidence remains untrusted until the verifier authenticates the
relevant committee and verifies the proof locally.

## Prerequisites

The examples require:

- Node.js 20 or later.
- network access to the public IOTA mainnet gRPC endpoint.
- network access to download the official mainnet `genesis.blob` on the first
  genesis-anchored run.

The examples use fixed targets from mainnet epoch 469 so they need no runtime
configuration. Public nodes may eventually prune this historical transaction
data. When that happens, replace the constants in
[`src/util.ts`](./src/util.ts) with related targets from one recent mainnet
transaction.

## Node.js

Install the dependencies and build the Package:

```bash
npm install
npm run build:nodejs
```

Run an example by passing its name to the shared example runner:

```bash
npm run example:node -- 01_create_and_verify_transaction_proof
```

## Examples

| Name | Information | Committee trust |
| :--- | :--- | :--- |
| [01_create_and_verify_transaction_proof](./src/01_create_and_verify_transaction_proof.ts) | Constructs a transaction proof, transfers it as JSON, and verifies it from mainnet genesis. | Mainnet genesis |
| [02_create_and_verify_multi_target_proof](./src/02_create_and_verify_multi_target_proof.ts) | Combines transaction, changed-object, and emitted-event targets in one proof. | Mainnet genesis |
| [03_reuse_verifier_for_multiple_proofs](./src/03_reuse_verifier_for_multiple_proofs.ts) | Reuses one verifier so authenticated committee history is not fetched and checked twice. | Mainnet genesis |
| [04_create_and_verify_object_proof](./src/04_create_and_verify_object_proof.ts) | Starts from an object ID and lets the builder discover its version and transaction. | Trusted node |
| [05_create_and_verify_event_proof](./src/05_create_and_verify_event_proof.ts) | Proves an event without declaring its emitting transaction as a separate target. | Trusted node |

## Example Workflow

The first example follows the complete transaction-proof lifecycle:

1. Connect to the public mainnet proof source.
2. Construct a proof for an existing transaction.
3. Serialize and deserialize the proof to model transfer between applications.
4. Download and locally cache the official mainnet genesis blob.
5. Authenticate committee transitions up to the checkpoint epoch.
6. Verify the proof locally.

The second example selects three related targets from one transaction. The
third demonstrates why applications should retain a verifier: the first
verification fills its authenticated in-memory committee cache and the second
reuses it. The fourth and fifth demonstrate target-driven discovery from an
object ID and event ID respectively.

> [!NOTE]
> Genesis-based committee resolution authenticates every transition from epoch
> 0 through epoch 469. A fresh verifier can therefore take a long time. The
> genesis blob cache at `~/.iota/iota_config/poi/mainnet/genesis.blob` avoids
> downloading the trust anchor again, but it does not cache committees.

## Trust Boundaries

- Selecting mainnet configures the source of proof material; it does not make
  that material trusted.
- Treat the complete proof payload as untrusted until verification succeeds.
- Obtain the genesis blob independently from the party supplying the proof and
  ensure it belongs to the proof's network.
- Retain a verifier when checking multiple proofs so it can reuse authenticated
  committees in memory.
- Examples 04 and 05 use trusted-node resolution. The connected node is inside
  the verifier's trust boundary in that mode.

The Rust examples also demonstrate persistent committee caching. The
TypeScript Package does not yet expose a caller-provided cache interface, so a
file-backed TypeScript example will be added with that API.
