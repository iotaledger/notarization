![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/banner_notarization.png)

<p align="center">
  <a href="https://iota.stackexchange.com/" style="text-decoration:none;"><img src="https://img.shields.io/badge/StackExchange-9cf.svg?logo=stackexchange" alt="StackExchange"></a>
  <a href="https://discord.gg/iota-builders" style="text-decoration:none;"><img src="https://img.shields.io/badge/Discord-9cf.svg?logo=discord" alt="Discord"></a>
  <img src="https://deps.rs/repo/github/iotaledger/notarization/status.svg" alt="Dependencies">
  <a href="https://github.com/iotaledger/notarization/blob/develop/LICENSE" style="text-decoration:none;"><img src="https://img.shields.io/github/license/iotaledger/notarization.svg" alt="Apache 2.0 license"></a>
</p>

<p align="center">
  <a href="#introduction">Introduction</a> ◈
  <a href="#packages">Packages</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#bindings">Bindings</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Notarization And Audit Trail

## Introduction

This repository contains two complementary IOTA ledger toolkits:

- **IOTA Notarization**
  Creates verifiable on-chain proof objects for arbitrary data, including dynamic and locked notarization flows.
- **IOTA Audit Trail**
  Creates shared on-chain audit trails with sequential records, role-based access control, locking, and tagging.

Each toolkit is split into:

- a Move package that defines the on-chain object model and behavior
- a Rust SDK that provides typed client access and transaction builders
- wasm bindings for JavaScript and TypeScript integrations

## Packages

| Toolkit | Move Package | Rust SDK | Wasm SDK |
| ------- | ------------ | -------- | -------- |
| Notarization | [`notarization-move`](./notarization-move) | [`notarization-rs`](./notarization-rs) | [`bindings/wasm/notarization_wasm`](./bindings/wasm/notarization_wasm) |
| Audit Trail | [`audit-trail-move`](./audit-trail-move) | [`audit-trail-rs`](./audit-trail-rs) | [`bindings/wasm/audit_trail_wasm`](./bindings/wasm/audit_trail_wasm) |

## Documentation And Resources

- IOTA Notarization:
  - [Notarization Rust SDK README](https://github.com/iotaledger/notarization/tree/main/notarization-rs/README.md)
  - [Notarization Wasm README](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/README.md)
  - [Notarization Examples](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/README.md)
  - [IOTA Notarization Docs Portal](https://docs.iota.org/developer/iota-notarization)
- IOTA Audit Trail:
  - [Audit Trail Rust SDK README](https://github.com/iotaledger/notarization/tree/main/audit-trail-rs/README.md)
  - [Audit Trail Move Package README](https://github.com/iotaledger/notarization/tree/main/audit-trail-move/README.md)
  - [Audit Trail Wasm README](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm/README.md)
  - [Audit Trail Wasm Examples](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm/examples/README.md)
- Shared:
  - [Repository Examples](https://github.com/iotaledger/notarization/tree/main/examples/README.md)

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) bindings in this repository:

- [Web Assembly](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm) for IOTA Notarization
- [Web Assembly](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/audit_trail_wasm) for IOTA Audit Trail

## Contributing

We would love to have you help us with the development of IOTA Notarization and Audit Trail. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these libraries or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
