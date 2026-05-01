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
  <a href="#suite-components">Suite Components</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#bindings">Bindings</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Notarization Suite

## Introduction

This repository contains the IOTA Notarization Suite, a set of IOTA ledger tools for verifiable on-chain data workflows.

The suite includes:

- **Single Notarization**
  Use this for individual locked or dynamic notarizations of arbitrary data, documents, hashes, or latest-state records.
- **Audit Trails**
  Use this for structured record histories with sequential entries, role-based access control, locking, and tagging.

Each suite component is available as:

- a **Move package** for the on-chain contracts
- a **Rust SDK** for typed client access and transaction builders
- **wasm bindings** for JavaScript and TypeScript integrations

## Where To Start

### I want a single notarized record

Use **Single Notarization** when your main need is proving the existence, integrity, or latest state of one notarized object on-chain.

- [Single Notarization Rust SDK](./notarization-rs)
- [Single Notarization Move Package](./notarization-move)
- [Single Notarization Wasm SDK](./bindings/wasm/notarization_wasm)
- [Single Notarization examples](./bindings/wasm/notarization_wasm/examples/README.md)

### I want an audit trail

Use **Audit Trails** when you need a structured record history with permissions, capabilities, tagging, and write or delete controls.

- [Audit Trails Rust SDK](./audit-trail-rs)
- [Audit Trails Move Package](./audit-trail-move)
- [Audit Trails Wasm SDK](./bindings/wasm/audit_trail_wasm)
- [Audit Trails examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### I want the on-chain contracts

- [Single Notarization Move](./notarization-move)
- [Audit Trails Move](./audit-trail-move)

### I want application SDKs

- [Single Notarization Rust](./notarization-rs)
- [Audit Trails Rust](./audit-trail-rs)
- [Single Notarization Wasm](./bindings/wasm/notarization_wasm)
- [Audit Trails Wasm](./bindings/wasm/audit_trail_wasm)

## Suite Components

| Component           | Best for                                                                    | Move Package                               | Rust SDK                               | Wasm SDK                                                 |
| ------------------- | --------------------------------------------------------------------------- | ------------------------------------------ | -------------------------------------- | -------------------------------------------------------- |
| Single Notarization | Individual locked or dynamic notarizations for documents, hashes, and state | [`notarization-move`](./notarization-move) | [`notarization-rs`](./notarization-rs) | [`notarization_wasm`](./bindings/wasm/notarization_wasm) |
| Audit Trails        | Shared sequential records with roles, capabilities, tagging, and locking    | [`audit-trail-move`](./audit-trail-move)   | [`audit-trail-rs`](./audit-trail-rs)   | [`audit_trail_wasm`](./bindings/wasm/audit_trail_wasm)   |

### Which one should I use?

| Need                                                                      | Best fit            |
| ------------------------------------------------------------------------- | ------------------- |
| Locked proof object for arbitrary data                                    | Single Notarization |
| Dynamic latest-state notarization flow                                    | Single Notarization |
| Shared sequential records with roles, capabilities, and record tag policy | Audit Trails        |
| Team or system audit log with governance and operational controls         | Audit Trails        |

## Documentation And Resources

### Single Notarization

- [Single Notarization Rust SDK README](./notarization-rs/README.md)
- [Single Notarization Move Package README](./notarization-move/README.md)
- [Single Notarization Wasm README](./bindings/wasm/notarization_wasm/README.md)
- [Single Notarization examples](./bindings/wasm/notarization_wasm/examples/README.md)
- [IOTA Notarization Docs Portal](https://docs.iota.org/developer/iota-notarization)

### Audit Trails

- [Audit Trails Rust SDK README](./audit-trail-rs/README.md)
- [Audit Trails Move Package README](./audit-trail-move/README.md)
- [Audit Trails Wasm README](./bindings/wasm/audit_trail_wasm/README.md)
- [Audit Trails examples](./bindings/wasm/audit_trail_wasm/examples/README.md)

### Shared

- [Repository examples](./examples/README.md)

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) bindings available in this repository:

- [Web Assembly for Single Notarization](./bindings/wasm/notarization_wasm)
- [Web Assembly for Audit Trails](./bindings/wasm/audit_trail_wasm)

## Contributing

We would love to have you help us with the development of the IOTA Notarization Suite. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about these libraries or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
