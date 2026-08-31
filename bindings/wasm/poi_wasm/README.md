# IOTA Proof of Inclusion Wasm Package

## Introduction

The Proof of Inclusion Wasm Package provides the Node.js and TypeScript interface for Proof of Inclusion in the IOTA
Notarization Toolkit. It connects a generated IOTA `LedgerService` client to `poi-rs` compiled as WebAssembly.

Use the Package to construct portable proofs for IOTA transactions, events, and object states and to verify those proofs
locally. `PoiClient` hides the generated protobuf client, ConnectRPC transport, and JavaScript-to-WASM source adapter,
while Rust owns proof construction, committee resolution, and verification.

Proof of Inclusion operates on existing ledger activity and does not define a separate Move Package.

## Installation

The Package currently builds from the repository. Install its development dependencies from this directory:

```bash
npm install
```

Node.js 24 or later, Rust 1.85 or later, `wasm-bindgen-cli`, and `wasm-opt` are required to build the Package.

## Client Creation

Create a client for a named public network or pass an explicit gRPC endpoint. The Package never selects a network
implicitly.

```ts
import { PoiClient } from "@iota/poi-wasm";

const mainnet = PoiClient.mainnet();
const testnet = PoiClient.testnet();
const devnet = PoiClient.devnet();
const custom = new PoiClient("http://localhost:50051");
```

Use an explicit endpoint for private nodes, archives, local networks, or alternative endpoints. Network selection
configures the source of proof material; it does not make the proof trusted.

## Proof Construction

`PoiClient.makeProof()` constructs one proof for transaction, object, and event targets that belong to the same
transaction.

```ts
import { PoiClient } from "@iota/poi-wasm";

const client = PoiClient.testnet();
const proof = await client.makeProof({
    transaction: transactionDigest,
    objects: [objectId],
    events: [{ transaction: transactionDigest, sequence: eventSequence }],
});

console.log(proof.toJSON());
```

The `transaction` field selects the transaction explicitly. The `objects` and `events` arrays select object and event
targets. Event sequence numbers and all other 64-bit values use JavaScript `bigint`.

The serialized proof records the targets explicitly selected by the caller. Its checkpoint summary and checkpoint
contents are sibling fields, while the required transaction proof contains the transaction, effects, and optional event
evidence. Object targets contain the selected object values, and event targets select events from the authenticated
transaction event list.

The JavaScript source adapter passes only opaque BCS bytes and checkpoint sequence numbers into WASM. Rust decodes those
values into existing IOTA domain types and delegates target resolution and proof construction to `poi-rs`.

## Verification

Create a verifier from the same `PoiClient`. Trusted-node resolution accepts the committee reported by a node already
inside the caller's trust boundary.

```ts
import { CommitteeResolution } from "@iota/poi-wasm";

const verifier = client.verifier(CommitteeResolution.trustedNode());
const verified = await verifier.verify(proof);
console.log(verified.transaction);
```

Use genesis-anchored resolution to authenticate committee lineage independently from the node:

```ts
import { readFile } from "node:fs/promises";

const trustedGenesisBlob = await readFile("genesis.blob");
const resolution = CommitteeResolution.fromGenesis(trustedGenesisBlob);
const verifier = client.verifier(resolution);

const verified = await verifier.verify(proof);
```

Successful verification returns a read-only `VerifiedProof`. It exposes the authenticated transaction digest,
checkpoint metadata, and `ProofTargets`. Use `objectBcs(index)` and `eventContents(index)` to read the authenticated
target payloads. Continue using `Proof` only as the untrusted transport and serialization envelope.

`CommitteeResolution.fromGenesis()` decodes the BCS-encoded IOTA genesis blob and extracts its committee in Rust.
Callers that already possess an extracted trusted committee can use `CommitteeResolution.anchored(committee)` instead.
`Committee.fromJSON()` accepts the Rust `Committee` fields `epoch` and `voting_rights`, validates public keys, rejects
duplicate authorities, requires total voting power to equal 10,000, and reconstructs the committee's derived lookup
state.

The verifier fetches the certified checkpoint in each epoch-close proof, verifies it with the current committee, and
only then accepts and caches the next committee. Each anchored verifier owns a fresh in-memory cache; the WASM Package
does not accept a caller-provided committee cache. Retain the verifier when checking multiple proofs so it can reuse the
committees authenticated during its lifetime. `CommitteeResolver.resolve(epoch)` and `Proof.verify(committee)` remain
available for lower-level committee resolution and offline verification; both verification methods return a
`VerifiedProof` on success.

## Trust Boundaries

Treat the node, source adapter, and complete proof payload as untrusted until verification succeeds. Trusted-node
resolution is appropriate only when the selected node is already an explicit trust anchor.

Obtain genesis blobs and extracted anchor committees independently from the party that supplies the proof. The proof's
`chain` value is informational and must not select the network, committee, genesis blob, or another trust anchor.

## Schema Workflow

[`grpc/iota-schema.lock.json`](grpc/iota-schema.lock.json) records the approved repository, exact Git revision, and
SHA-256 digest of the committed Buf image. Normal generation does not access the network.

Download a different upstream schema only as an intentional update:

```bash
npm run grpc:schema:update -- <full-iota-rust-sdk-commit>
```

Regenerate the TypeScript client from the committed schema image:

```bash
npm run grpc:generate
```

Review the lock file, Buf image, and generated TypeScript changes together. The generated client uses Protobuf-ES
messages and service descriptors with ConnectRPC's native Node.js gRPC transport over HTTP/2.

## Development And Testing

Build the Node.js Package:

```bash
npm run build
```

Regenerate the client, build `poi-rs` for `wasm32-unknown-unknown`, type-check the TypeScript boundary, and run the unit
tests:

```bash
npm run verify
```

The unit tests use an in-memory generated service implementation and do not require a running IOTA node.

## Examples

The [Proof of Inclusion Wasm Examples](./examples/README.md) cover transaction, multi-target, verifier-reuse, object,
and event proofs against the active IOTA CLI environment.

## Documentation And Resources

- [Proof of Inclusion Rust Package](../../../poi-rs/README.md)
- [Proof of Inclusion Rust Examples](../../../examples/poi/README.md)
- [Proof of Inclusion Wasm Examples](./examples/README.md)
- [Repository Root](../../../README.md)

## Contributing

We would love to have you help us develop the IOTA Notarization Toolkit. Every contribution is greatly valued.

Review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the
[IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, fork the project, push your changes to your fork, and create a pull request.

Join the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders) for development discussions and
support. You can also ask questions on [IOTA Stack Exchange](https://iota.stackexchange.com/).
