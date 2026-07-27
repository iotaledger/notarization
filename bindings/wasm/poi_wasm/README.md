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

## Source usage

```ts
import { NodePoiSource } from "./src/index.js";

const source = new NodePoiSource("https://grpc.testnet.iota.cafe:443");
const chainIdentifier = await source.chainIdentifier();
const transaction = await source.transaction(transactionDigest);
const object = await source.object(objectId, version);
const checkpoint = await source.checkpoint(42n);
```

`NodePoiSource` returns only opaque BCS bytes and checkpoint sequence numbers.
The WASM `Source` adapter decodes those values into existing IOTA Rust types,
then delegates target resolution and proof construction to `poi-rs`.

## Proof construction

```ts
import { NodePoiSource, ProofBuilder } from "./src/index.js";

const source = new NodePoiSource("https://grpc.testnet.iota.cafe:443");
const proof = await new ProofBuilder(source)
  .transaction(transactionDigest)
  .build();

console.log(proof.toJSON());
```

The same builder also exposes `object(objectId)` and
`event(transactionDigest, eventSequence)`. All 64-bit values use JavaScript
`bigint`.

The lower-level generated client remains available through
`createIotaGrpcClient` when direct access to another `LedgerService` method is
needed.

## Verification

```sh
npm install
npm run verify
```

Verification regenerates the Node.js protobuf client from the committed schema
image, builds `poi-rs` for `wasm32-unknown-unknown`, type-checks the TypeScript
boundary, and runs the tests. The tests use an in-memory generated service
implementation and do not require a running IOTA node. To query a live
endpoint:

```sh
npm run example:service-info -- https://grpc.testnet.iota.cafe:443
```
