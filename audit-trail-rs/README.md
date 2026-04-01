![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/banner_notarization.png)

<p align="center">
  <a href="https://iota.stackexchange.com/" style="text-decoration:none;"><img src="https://img.shields.io/badge/StackExchange-9cf.svg?logo=stackexchange" alt="StackExchange"></a>
  <a href="https://discord.gg/iota-builders" style="text-decoration:none;"><img src="https://img.shields.io/badge/Discord-9cf.svg?logo=discord" alt="Discord"></a>
  <a href="https://github.com/iotaledger/notarization/blob/develop/LICENSE" style="text-decoration:none;"><img src="https://img.shields.io/github/license/iotaledger/notarization.svg" alt="Apache 2.0 license"></a>
</p>

<p align="center">
  <a href="#introduction">Introduction</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#feature-overview">Feature Overview</a> ◈
  <a href="#bindings">Bindings</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Audit Trail Rust SDK

## Introduction

`audit_trail` is the Rust SDK for reading and writing audit trails on the IOTA ledger.

An audit trail is a shared on-chain object that stores a sequential series of records together with:

- role-based access control backed by capabilities
- trail-level locking rules for writes and deletions
- tag registries for record categorization
- immutable creation metadata and optional updatable metadata

The crate provides:

- read-only and signing client wrappers for the on-chain audit-trail package
- typed trail handles for records, locking, access control, and tags
- serializable Rust representations of on-chain objects and emitted events
- transaction builders that integrate with the shared `product_common` transaction flow

## Documentation And Resources

- [Audit Trail Move Package](https://github.com/iotaledger/notarization/tree/main/audit-trail-move): On-chain contract package that defines the shared object model, permissions, locking, and events.
- [Wasm SDK](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm): JavaScript and TypeScript bindings for browser and Node.js integrations.
- [Wasm Examples](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm/examples/README.md): Runnable audit-trail examples for JS and TS consumers.
- [Repository Examples](https://github.com/iotaledger/notarization/tree/main/examples/README.md): End-to-end examples across the broader repository.

This README is also used as the crate-level rustdoc entry point, while the source files provide detailed API documentation for all public types and methods.

## Feature Overview

The public API is organized around a small set of entry points:

- [`AuditTrailClientReadOnly`] for package resolution, trail-scoped reads, and inspected transactions
- [`AuditTrailClient`] for signed write flows
- [`AuditTrailHandle`] for operations scoped to one trail object
- [`AuditTrailBuilder`] for configuring trail creation
- [`core::types`] for domain types such as [`Data`], [`Record`], [`LockingConfig`], and [`PermissionSet`]

Typical flow:

1. Construct an [`AuditTrailClientReadOnly`] or [`AuditTrailClient`].
2. Resolve a trail with [`AuditTrailClientReadOnly::trail`] or [`AuditTrailClient::trail`].
3. Read state with [`AuditTrailHandle::get`] or move into one of the trail subsystems:
   - [`AuditTrailHandle::records`]
   - [`AuditTrailHandle::locking`]
   - [`AuditTrailHandle::access`]
   - [`AuditTrailHandle::tags`]
4. For writes, build a typed transaction from the client, trail handle, or subsystem handle and execute it through the surrounding transaction infrastructure.

The crate deliberately separates transaction construction from submission so applications can keep signing, sponsorship, gas selection, and batching policy outside the SDK.

Pure value types expose executable doctests where the behavior is self-contained and stable:

```rust
use audit_trail::core::types::{Data, InitialRecord};

let record = InitialRecord::new(Data::text("hello"), Some("first write".to_string()), None);

assert_eq!(record.data, Data::text("hello"));
assert_eq!(record.metadata.as_deref(), Some("first write"));
assert!(record.tag.is_none());
```

If you are integrating against a custom deployment, use [`PackageOverrides`] during client construction so the crate does not rely on the built-in package registry for that environment.

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) bindings of this Rust SDK to other programming languages:

- [Web Assembly](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm) (JavaScript/TypeScript)

## Contributing

We would love to have you help us with the development of IOTA Audit Trail. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about this library or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).