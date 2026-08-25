![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/banner_notarization.png)

<p align="center">
  <a href="https://iota.stackexchange.com/" style="text-decoration:none;"><img src="https://img.shields.io/badge/StackExchange-9cf.svg?logo=stackexchange" alt="StackExchange"></a>
  <a href="https://discord.gg/iota-builders" style="text-decoration:none;"><img src="https://img.shields.io/badge/Discord-9cf.svg?logo=discord" alt="Discord"></a>
  <img src="https://deps.rs/repo/github/iotaledger/notarization/status.svg" alt="Dependencies">
  <a href="https://github.com/iotaledger/notarization/blob/develop/LICENSE" style="text-decoration:none;"><img src="https://img.shields.io/github/license/iotaledger/notarization.svg" alt="Apache 2.0 license"></a>
</p>

<p align="center">
  <a href="#introduction">Introduction</a> ◈
  <a href="#where-to-start">Where To Start</a> ◈
  <a href="#packages">Packages</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Notarization Toolkit

## Introduction

This repository contains the IOTA Notarization Toolkit, a set of IOTA ledger tools for verifiable on-chain data workflows.

The toolkit includes:

- **Single Notarization**
  Use this for individual locked or dynamic notarizations of arbitrary data, documents, hashes, or latest-state records.
- **Audit Trails**
  Use this for structured record histories with sequential entries, role-based access control, locking, and tagging.
- **Proof of Inclusion**
  Use this to construct portable cryptographic evidence that a transaction, event, or object state is included in a
  certified IOTA checkpoint, and to verify that evidence locally.

## Where To Start

Use [Single Notarization](#single-notarization) for one on-chain object that stores arbitrary data, a document hash, or
the latest state of a record.

Use [Audit Trails](#audit-trails) for structured record histories with permissions, capabilities, tagging, and write or
delete controls.

Use [Proof of Inclusion](#proof-of-inclusion) when a verifier needs portable evidence that a transaction, event, or
object state is included in a certified checkpoint. Proof of Inclusion verifies existing ledger activity and does not
define a separate on-chain object or Move Package.

## Packages

Single Notarization and Audit Trails provide on-chain contracts:

- [Single Notarization Move Package](./notarization-move)
- [Audit Trails Move Package](./audit-trail-move)

All three components provide Rust and TypeScript/JavaScript packages:

- [Single Notarization Rust Package](./notarization-rs)
- [Audit Trails Rust Package](./audit-trail-rs)
- [Proof of Inclusion Rust Package](./poi-rs)
- [Single Notarization Wasm Package](./bindings/wasm/notarization_wasm)
- [Audit Trails Wasm Package](./bindings/wasm/audit_trail_wasm)
- [Proof of Inclusion Wasm Package](./bindings/wasm/poi_wasm)

## Documentation and Resources

- [IOTA Notarization documentation](https://docs.iota.org/developer/iota-notarization/)

### Single Notarization

- [Rust Package README](./notarization-rs/README.md)
- [Rust API documentation](https://iotaledger.github.io/notarization/notarization/index.html)
- [Rust Examples](./examples/README.md)
- [Move Package README](./notarization-move/README.md)
- [Wasm Package README](./bindings/wasm/notarization_wasm/README.md)
- [Wasm Examples](./bindings/wasm/notarization_wasm/examples/README.md)

### Audit Trails

- [Rust Package README](./audit-trail-rs/README.md)
- [Rust API documentation](https://iotaledger.github.io/notarization/audit_trails/index.html)
- [Rust Examples](./examples/audit-trail/README.md)
- [Move Package README](./audit-trail-move/README.md)
- [Wasm Package README](./bindings/wasm/audit_trail_wasm/README.md)
- [Wasm Examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### Proof of Inclusion

- [Rust Package README](./poi-rs/README.md)
- [Rust API documentation](https://iotaledger.github.io/notarization/poi_rs/index.html)
- [Rust Examples](./examples/poi/README.md)
- [Wasm Package README](./bindings/wasm/poi_wasm/README.md)
- [Wasm Examples](./bindings/wasm/poi_wasm/examples/README.md)

## Contributing

We would love to have you help us with the development of the IOTA Notarization Toolkit. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these Packages or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
