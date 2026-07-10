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

See [Single Notarization resources](#single-notarization) for one on-chain proof object, such as a document hash, immutable record, or dynamic latest-state record.

See [Audit Trails resources](#audit-trails) for structured record histories with permissions, capabilities, tagging, and write or delete controls.

If you need to integrate the Notarization on-chain contracts into your own Move Package:

- [Notarization Move Package](./notarization-move)
- [Audit Trails Move Package](./audit-trail-move)

If you want to build a client application:

- [Rust client for notarized records](./notarization-rs)
- [Rust client for audit trails](./audit-trail-rs)
- [Wasm bindings for notarized records](./bindings/wasm/notarization_wasm)
- [Wasm bindings for audit trails](./bindings/wasm/audit_trail_wasm)

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

## Contributing

We would love to have you help us with the development of the IOTA Notarization Toolkit. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these libraries or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
