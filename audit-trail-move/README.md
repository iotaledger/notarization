![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/banner_notarization.png)

<p align="center">
  <a href="https://iota.stackexchange.com/" style="text-decoration:none;"><img src="https://img.shields.io/badge/StackExchange-9cf.svg?logo=stackexchange" alt="StackExchange"></a>
  <a href="https://discord.gg/iota-builders" style="text-decoration:none;"><img src="https://img.shields.io/badge/Discord-9cf.svg?logo=discord" alt="Discord"></a>
  <a href="https://github.com/iotaledger/notarization/blob/develop/LICENSE" style="text-decoration:none;"><img src="https://img.shields.io/github/license/iotaledger/notarization.svg" alt="Apache 2.0 license"></a>
</p>

<p align="center">
  <a href="#introduction">Introduction</a> ◈
  <a href="#modules">Modules</a> ◈
  <a href="#development-and-testing">Development & Testing</a> ◈
  <a href="#related-libraries">Related Libraries</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Audit Trail Move Package

## Introduction

`audit-trail-move` is the on-chain Move package behind IOTA Audit Trail.

It defines the shared `AuditTrail` object and the supporting types needed for:

- sequential record storage
- role-based access control through capabilities
- trail-wide locking for writes and deletions
- record tags and role tag restrictions
- immutable and updatable trail metadata
- emitted events for trail and record lifecycle changes

The package depends on `TfComponents` for reusable capability, role-map, and timelock primitives.

## Modules

- `audit_trail::main`
  Core shared object, events, trail lifecycle, record mutation, metadata updates, roles, and capabilities.
- `audit_trail::record`
  Record payloads, initial records, and correction metadata.
- `audit_trail::locking`
  Locking configuration and lock evaluation helpers.
- `audit_trail::permission`
  Permission constructors and admin permission presets.
- `audit_trail::record_tags`
  Tag registry and role tag helpers.

## Development And Testing

Build the Move package:

```bash
cd audit-trail-move
iota move build
```

Run the Move test suite:

```bash
cd audit-trail-move
iota move test
```

Publish locally:

```bash
cd audit-trail-move
./scripts/publish_package.sh
```

The publish script prints `IOTA_AUDIT_TRAIL_PKG_ID` and, on `localnet`, also exports `IOTA_TF_COMPONENTS_PKG_ID`.

The package history files [`Move.lock`](./Move.lock) and [`Move.history.json`](./Move.history.json) are used by the Rust SDK to resolve and track deployed package versions.

## Related Libraries

- [Rust SDK](https://github.com/iotaledger/notarization/tree/main/audit-trail-rs/README.md)
- [Wasm SDK](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm/README.md)
- [Repository Root](https://github.com/iotaledger/notarization/tree/main/README.md)

## Contributing

We would love to have you help us with the development of IOTA Audit Trail. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about this package or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
