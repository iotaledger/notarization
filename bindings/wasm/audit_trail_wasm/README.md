# `audit_trail_wasm`

`audit_trail_wasm` exposes the `audit_trails` Rust crate to JavaScript and TypeScript consumers through `wasm-bindgen`.

It is designed for browser and other `wasm32` environments that need:

- read-only and signing audit-trail clients
- typed wrappers for trail handles, records, locking, access control, and tags
- serializable value and event types that map cleanly into JS/TS
- transaction wrappers that integrate with the shared `product_common` wasm transaction helpers

## Main entry points

- `AuditTrailClientReadOnly` for reads and inspected transactions
- `AuditTrailClient` for signed write flows
- `AuditTrailBuilder` for creating new trails
- `AuditTrailHandle` for trail-scoped APIs
- `TrailRecords`, `TrailLocking`, `TrailAccess`, and `TrailTags` for subsystem-specific operations

## Choosing an entry point

- Use `AuditTrailClientReadOnly` when you need reads, package resolution, or inspected transactions.
- Use `AuditTrailClient` when you also need typed write transaction builders.
- Use `AuditTrailHandle` after you already know the trail object ID and want to stay scoped to that trail.
- Use `AuditTrailBuilder` when you are preparing a create-trail transaction.

## Data model wrappers

The bindings expose JS-friendly wrappers for the most important Rust value types:

- `Data`
- `Permission` and `PermissionSet`
- `RoleTags`, `RoleMap`, and `CapabilityIssueOptions`
- `TimeLock`, `LockingWindow`, and `LockingConfig`
- `Record`, `PaginatedRecord`, and `OnChainAuditTrail`
- event payloads such as `RecordAdded`, `RoleCreated`, and `CapabilityIssued`

## Typical read flow

1. Create an `AuditTrailClientReadOnly` or `AuditTrailClient`.
2. Resolve a trail handle with `.trail(trailId)`.
3. Read state with `.get()`, `.records().get(...)`, `.records().listPage(...)`, or `.locking().isRecordLocked(...)`.

## Typical write flow

1. Create an `AuditTrailClient` with a transaction signer.
2. Build a transaction from `client.createTrail()`, `client.trail(trailId)`, or one of the trail subsystem handles.
3. Convert that transaction wrapper into programmable transaction bytes.
4. Submit it through your surrounding JS transaction flow and feed the effects and events back into the typed `applyWithEvents(...)` helper.

The package intentionally separates transaction construction from submission so browser apps, wallet integrations, and server-side signing flows can keep transport and execution policy outside the package.

## Minimal TypeScript shape

```ts
import { AuditTrailClientReadOnly } from "@iota/audit-trails";

const client = await AuditTrailClientReadOnly.create(iotaClient);
const trail = client.trail(trailId);
const state = await trail.get();

console.log(state.sequenceNumber);
```

## Build the Library

Alternatively, you can build the bindings yourself if you have Rust installed. If not, refer
to [rustup.rs](https://rustup.rs) for the installation.

### Requirements

- [Node.js](https://nodejs.org/en) (>= `v20`)
- [Rust](https://www.rust-lang.org/) (>= 1.65)
- [Cargo](https://doc.rust-lang.org/cargo/) (>= 1.65)
- for running example: a local network node with the IOTA Audit Trails Package deployed as described in [Local Network Setup](https://docs.iota.org/developer/iota-notarization/audit-trails/getting-started/local-network-setup)

### 1. Install Local Tooling

If you want to build the library from source you have to install additional build tools locally.

#### Install `wasm-bindgen-cli`

First you need to install [`wasm-bindgen-cli`](https://github.com/rustwasm/wasm-bindgen).
A manual installation is required because we use the [Weak References](https://rustwasm.github.io/wasm-bindgen/reference/weak-references.html) feature,
which [`wasm-pack` does not expose](https://github.com/rustwasm/wasm-pack/issues/930).

```bash
cargo install --force wasm-bindgen-cli
```

#### Install `wasm-opt`

To reduce the size of the wasm package, it is optimized with `wasm-opt`, which is part of [`binaryen`](https://github.com/WebAssembly/binaryen).

You can either download a [release of binaryen](https://github.com/WebAssembly/binaryen/releases) and make the bin folder available in your PATH or check if your operating system tooling offers a more convenient way of installing the binaries like APT, Homebrew, etc.

Some examples:

- Linux via APT: `sudo apt-get update && sudo apt-get -y install binaryen` (taken from [here](https://installati.one/install-binaryen-ubuntu-22-04/))
- MacOS via Homebrew: `brew install binaryen` (see [Homebrew entry](https://formulae.brew.sh/formula/binaryen))

### 2. Install Dependencies

After installing local tooling, you can install the necessary dependencies using the following command:

```bash
npm install
```

### 3. Build

You can build the bindings for `node.js` using the following command:

```bash npm2yarn
npm run build
```

## Examples

See [examples/README.md](./examples/README.md) for runnable node and web example flows.
