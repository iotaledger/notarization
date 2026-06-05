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
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#bindings">Bindings</a> ◈
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

Each toolkit component is available as:

- **Move Package** for the on-chain contracts
- **Rust Package** for typed client access and transaction builders
- **TypeScript/JS Package** using wasm bindings for the above-mentioned Rust package

## Where To Start

### I want a single notarized record

Choose this path for one on-chain proof object, such as a document hash, immutable record, or dynamic latest-state record.

- [Rust client](./notarization-rs)
- [Move contracts](./notarization-move)
- [Wasm bindings](./bindings/wasm/notarization_wasm)
- [Examples](./bindings/wasm/notarization_wasm/examples/README.md)

### I want an audit trail

Choose this path for structured record histories with permissions, capabilities, tagging, and write or delete controls.

- [Rust client](./audit-trail-rs)
- [Move contracts](./audit-trail-move)
- [Wasm bindings](./bindings/wasm/audit_trail_wasm)
- [Examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### I want the on-chain contracts

- [Notarization contracts](./notarization-move)
- [Audit trail contracts](./audit-trail-move)

### I want to build an application

- [Rust client for notarized records](./notarization-rs)
- [Rust client for audit trails](./audit-trail-rs)
- [Wasm bindings for notarized records](./bindings/wasm/notarization_wasm)
- [Wasm bindings for audit trails](./bindings/wasm/audit_trail_wasm)

## Documentation and Resources

### Single Notarization

- [Rust package README](./notarization-rs/README.md)
- [Rust API documentation](https://iotaledger.github.io/notarization/notarization/index.html)
- [Move package README](./notarization-move/README.md)
- [Wasm package README](./bindings/wasm/notarization_wasm/README.md)
- [Examples](./bindings/wasm/notarization_wasm/examples/README.md)
- [Docs portal](https://docs.iota.org/developer/iota-notarization)

### Audit Trails

- [Rust package README](./audit-trail-rs/README.md)
- [Rust API documentation](https://iotaledger.github.io/notarization/audit_trails/index.html)
- [Move package README](./audit-trail-move/README.md)
- [Wasm package README](./bindings/wasm/audit_trail_wasm/README.md)
- [Examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### Shared

- [Repository examples](./examples/README.md)

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) bindings available in this repository:

- [Web Assembly for Single Notarization](./bindings/wasm/notarization_wasm)
- [Web Assembly for Audit Trails](./bindings/wasm/audit_trail_wasm)

## Contributing

We would love to have you help us with the development of the IOTA Notarization Toolkit. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these libraries or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
