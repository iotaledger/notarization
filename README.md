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
  <a href="#toolkits">Toolkits</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#bindings">Bindings</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Notarization And Audit Trail

## Introduction

This repository contains two complementary IOTA ledger toolkits for verifiable on-chain data workflows:

- **IOTA Notarization**
  Best when you want a proof object for arbitrary data, documents, hashes, or latest-state notarization flows.
- **IOTA Audit Trail**
  Best when you want shared audit records with sequential entries, role-based access control, locking, and tagging.

Each toolkit is available as:

- a **Move package** for the on-chain contracts
- a **Rust SDK** for typed client access and transaction builders
- **wasm bindings** for JavaScript and TypeScript integrations

## Where To Start

### I want to notarize data

Use **IOTA Notarization** when your main need is proving the existence, integrity, or latest state of data on-chain.

- [Notarization Rust SDK](./notarization-rs)
- [Notarization Move Package](./notarization-move)
- [Notarization Wasm SDK](./bindings/wasm/notarization_wasm)
- [Notarization examples](./bindings/wasm/notarization_wasm/examples/README.md)

### I want audit records

Use **IOTA Audit Trail** when you need shared audit records with permissions, capabilities, tagging, and write or delete controls.

- [Audit Trail Rust SDK](./audit-trail-rs)
- [Audit Trail Move Package](./audit-trail-move)
- [Audit Trail Wasm SDK](./bindings/wasm/audit_trail_wasm)
- [Audit Trail examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### I want the on-chain contracts

- [Notarization Move](./notarization-move)
- [Audit Trail Move](./audit-trail-move)

### I want application SDKs

- [Notarization Rust](./notarization-rs)
- [Audit Trail Rust](./audit-trail-rs)
- [Notarization Wasm](./bindings/wasm/notarization_wasm)
- [Audit Trail Wasm](./bindings/wasm/audit_trail_wasm)

## Toolkits

| Toolkit      | Best for                                                                 | Move Package                               | Rust SDK                               | Wasm SDK                                                 |
| ------------ | ------------------------------------------------------------------------ | ------------------------------------------ | -------------------------------------- | -------------------------------------------------------- |
| Notarization | Proof objects for documents, hashes, and updatable notarized state       | [`notarization-move`](./notarization-move) | [`notarization-rs`](./notarization-rs) | [`notarization_wasm`](./bindings/wasm/notarization_wasm) |
| Audit Trail  | Shared sequential records with roles, capabilities, tagging, and locking | [`audit-trail-move`](./audit-trail-move)   | [`audit-trail-rs`](./audit-trail-rs)   | [`audit_trail_wasm`](./bindings/wasm/audit_trail_wasm)   |

### Which one should I use?

| Need                                                                      | Best fit     |
| ------------------------------------------------------------------------- | ------------ |
| Immutable or updatable proof object for arbitrary data                    | Notarization |
| Simple proof-of-existence or latest-state notarization flow               | Notarization |
| Shared sequential records with roles, capabilities, and record tag policy | Audit Trail  |
| Team or system audit log with governance and operational controls         | Audit Trail  |

## Documentation And Resources

### IOTA Notarization

- [Notarization Rust SDK README](./notarization-rs/README.md)
- [Notarization Move Package README](./notarization-move/README.md)
- [Notarization Wasm README](./bindings/wasm/notarization_wasm/README.md)
- [Notarization examples](./bindings/wasm/notarization_wasm/examples/README.md)
- [IOTA Notarization Docs Portal](https://docs.iota.org/developer/iota-notarization)

### IOTA Audit Trail

- [Audit Trail Rust SDK README](./audit-trail-rs/README.md)
- [Audit Trail Move Package README](./audit-trail-move/README.md)
- [Audit Trail Wasm README](./bindings/wasm/audit_trail_wasm/README.md)
- [Audit Trail examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### Shared

- [Repository examples](./examples/README.md)

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) bindings available in this repository:

- [Web Assembly for IOTA Notarization](./bindings/wasm/notarization_wasm)
- [Web Assembly for IOTA Audit Trail](./bindings/wasm/audit_trail_wasm)

## Contributing

We would love to have you help us with the development of IOTA Notarization and Audit Trail. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these libraries or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
