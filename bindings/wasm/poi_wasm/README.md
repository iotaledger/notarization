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
```

No network is selected implicitly. The named constructors use the public IOTA
gRPC endpoints.

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
const resolver = client.committeeResolver();
const committee = await resolver.resolve(proof.checkpointEpoch);

proof.verify(committee);
```

`CommitteeResolver` asks the client's node for the committee governing the
proof checkpoint epoch. Rust validates the returned committee representation
and performs proof verification locally with `poi-rs`.

This mode places the node inside the caller's trust boundary. It does not
authenticate committee lineage from genesis. Genesis-anchored committee
resolution will be added separately.

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
