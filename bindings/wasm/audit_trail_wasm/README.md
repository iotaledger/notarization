# `audit_trail_wasm`

`audit_trail_wasm` exposes the `audit_trail` Rust SDK to JavaScript and TypeScript consumers through `wasm-bindgen`.

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

The bindings intentionally separate transaction construction from submission so browser apps, wallet integrations, and server-side signing flows can keep transport and execution policy outside the SDK.

## Minimal TypeScript shape

```ts
import { AuditTrailClientReadOnly } from "@iota/audit-trail-wasm";

const client = await AuditTrailClientReadOnly.create(iotaClient);
const trail = client.trail(trailId);
const state = await trail.get();

console.log(state.sequenceNumber);
```

## Build

```bash
npm install
npm run build
```

## Examples

See [examples/README.md](./examples/README.md) for runnable node and web example flows.
