# IOTA Audit Trails WASM Library

`audit_trails_wasm` provides the Rust-to-WASM bindings for the `audit_trails` crate and is published to JavaScript consumers as `@iota/audit-trails`.

The current MVP surface includes:

- `AuditTrailClientReadOnly`
- `AuditTrailClient`
- `AuditTrailBuilder`
- `AuditTrailHandle`
- `TrailRecords`
- `Data`
- `Record`
- `PaginatedRecord`
- `OnChainAuditTrail`
- `ImmutableMetadata`
- `LockingConfig`
- `LockingWindow`
- `TimeLock`

## Build

```bash
npm install
npm run build
```

## Examples

See [examples/README.md](./examples/README.md) for the node example flows.
