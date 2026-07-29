# Proof of Inclusion Node.js Package

This package generates a typed Node.js client for IOTA's `LedgerService` and
connects it to `poi-rs` compiled as WebAssembly.

The generated client uses:

- protobuf definitions pinned to the same `iota-rust-sdk` revision as the Rust
  workspace;
- Protobuf-ES generated messages and service descriptors;
- ConnectRPC's native Node.js gRPC transport over HTTP/2.

## Schema workflow

[`grpc/iota-schema.lock.json`](grpc/iota-schema.lock.json) records the approved
repository, exact Git revision and SHA-256 digest of the committed Buf image.
Normal generation does not access the network.

To intentionally download a different upstream schema:

```sh
npm run grpc:schema:update -- <full-iota-rust-sdk-commit>
```

To regenerate the TypeScript client from the committed schema image:

```sh
npm run grpc:generate
```

Review the lock file, Buf image and generated TypeScript changes together.

## Client creation

```ts
import { PoiClient } from "@iota/poi-wasm";

const mainnet = PoiClient.mainnet();
const testnet = PoiClient.testnet();
const devnet = PoiClient.devnet();
const custom = new PoiClient("http://localhost:9000");
```

No network is selected implicitly. The named constructors use the public IOTA
gRPC endpoints. Construct `PoiClient` with an explicit endpoint for private
nodes, archives, local networks, or alternative endpoints.

## Proof construction

```ts
import { PoiClient } from "@iota/poi-wasm";

const client = PoiClient.testnet();
const proof = await client
  .proof()
  .transaction(transactionDigest)
  .build();

console.log(proof.toJSON());
```

The same builder also exposes `object(objectId)` and
`event(transactionDigest, eventSequence)`. All 64-bit values use JavaScript
`bigint`.

`PoiClient` hides the generated protobuf client, gRPC transport, and
JavaScript/WASM source adapter. The adapter passes only opaque BCS bytes and
checkpoint sequence numbers into WASM. Rust decodes those values into existing
IOTA domain types and delegates target resolution and proof construction to
`poi-rs`.

## Trusted-node verification

```ts
const verifier = client.trustedNodeVerifier();
await verifier.verify(proof);
```

The verifier asks the client's node for the committee governing the proof
checkpoint epoch. Rust validates the returned committee representation and
performs proof verification locally with `poi-rs`.

This mode places the node inside the caller's trust boundary. It does not
authenticate committee lineage from genesis. To authenticate committee
lineage from an already trusted committee:

```ts
import { readFile } from "node:fs/promises";
import { Committee } from "@iota/poi-wasm";

const committeeJson = await readFile("trusted-committee.json", "utf8");
const trustedGenesisCommittee = Committee.fromJSON(committeeJson);
const verifier = client.anchoredVerifier(trustedGenesisCommittee);

await verifier.verify(proof);
```

The committee JSON uses the Rust `Committee` fields `epoch` and
`voting_rights`. Rust/WASM validates public keys, rejects duplicate authorities,
requires total voting power to equal 10,000, and reconstructs the committee's
derived lookup state.

The verifier fetches the certified checkpoint in each epoch-close proof,
verifies it with the current committee, and only then accepts and caches the
next committee. The node supplies evidence but is not trusted to choose the
committee.

Retain the verifier when checking multiple proofs so its authenticated
committee cache is reused. `CommitteeResolver.resolve(epoch)` and
`Proof.verify(committee)` remain available for callers that need the
lower-level committee or offline-verification APIs.

## Package verification

```sh
npm install
npm run verify
```

Verification regenerates the Node.js protobuf client from the committed schema
image, builds `poi-rs` for `wasm32-unknown-unknown`, type-checks the TypeScript
boundary, and runs the tests. The tests use an in-memory generated service
implementation and do not require a running IOTA node. To query a live
endpoint with the development diagnostic:

```sh
npm run example:service-info -- https://grpc.testnet.iota.cafe:443
```
