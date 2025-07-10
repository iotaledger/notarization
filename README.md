![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/notarization.svg)

<p align="center">
  <a href="https://iota.stackexchange.com/" style="text-decoration:none;"><img src="https://img.shields.io/badge/StackExchange-9cf.svg?logo=stackexchange" alt="StackExchange"></a>
  <a href="https://discord.gg/iota-builders" style="text-decoration:none;"><img src="https://img.shields.io/badge/Discord-9cf.svg?logo=discord" alt="Discord"></a>
  <img src="https://deps.rs/repo/github/iotaledger/notarization/status.svg" alt="Dependencies">
  <a href="https://github.com/iotaledger/notarization/blob/develop/LICENSE" style="text-decoration:none;"><img src="https://img.shields.io/github/license/iotaledger/notarization.svg" alt="Apache 2.0 license"></a>
</p>

<p align="center">
  <a href="#introduction">Introduction</a> ◈
  <a href="#documentation-and-resources">Documentation & Resources</a> ◈
  <a href="#bindings">Bindings</a> ◈
  <a href="#contributing">Contributing</a>
</p>

---

# IOTA Notarization

## Introduction

IOTA Notarization enables the creation of immutable, on-chain records for any arbitrary data. This is achieved by storing the data, or a hash of it, inside a dedicated Move object on the IOTA ledger. This process provides a verifiable, timestamped proof of the data's existence and integrity at a specific point in time.

IOTA Notarization is composed of two primary components:

- **Notarization Move Package**: The on-chain smart contracts that define the behavior and structure of notarization objects.
- **Notarization Library (Rust/Wasm)**: A client-side library that provides developers with convenient functions to create, manage, and verify `Notarization` objects on the network.

## Documentation and Resources

- [Notarization Documentation Pages](https://docs.iota.org/developer/iota-notarization): Supplementing documentation with context around notarization and simple examples on library usage.
- API References:
  - [Rust API Reference](https://iotaledger.github.io/notarization/notarization/index.html): Package documentation (cargo docs).

<!--  - [Wasm API Reference](https://docs.iota.org/references/iota-notarization/wasm/api_ref): Wasm Package documentation. -->

- Examples:
  - [Rust Examples](https://github.com/iotaledger/notarization/tree/main/examples/README.md): Practical code snippets to get you started with the library in Rust.
  - [Wasm Examples](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/README.md): Practical code snippets to get you started with the library in TypeScript/JavaScript.

## Bindings

[Foreign Function Interface (FFI)](https://en.wikipedia.org/wiki/Foreign_function_interface) Bindings of this [Rust](https://www.rust-lang.org/) library to other programming languages:

- [Web Assembly](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm) (JavaScript/TypeScript)

## Contributing

We would love to have you help us with the development of IOTA Notarization. Each and every contribution is greatly valued!

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included!

The best place to get involved in discussions about this library or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
