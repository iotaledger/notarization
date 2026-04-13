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

# IOTA Notarization Move Package

## Introduction

`notarization-move` is the on-chain Move package behind IOTA Notarization.

It defines the core `Notarization` object and the supporting modules for:

- dynamic notarization flows
- locked notarization flows
- immutable creation metadata
- optional updatable metadata
- state updates, transfer rules, and destruction checks
- emitted events for notarization lifecycle changes

The package depends on `TfComponents` for shared timelock primitives.

## Modules

- `iota_notarization::notarization`
  Core object, state model, metadata, lock metadata, updates, and destruction logic.
- `iota_notarization::dynamic_notarization`
  Dynamic notarization creation and transfer flows.
- `iota_notarization::locked_notarization`
  Locked notarization creation flows with timelock controls.
- `iota_notarization::method`
  Method discriminator helpers for dynamic and locked variants.

## Development And Testing

Build the Move package:

```bash
cd notarization-move
iota move build
```

Run the Move test suite:

```bash
cd notarization-move
iota move test
```

Publish locally:

```bash
cd notarization-move
./scripts/publish_package.sh
```

The package history files [`Move.lock`](./Move.lock) and [`Move.history.json`](./Move.history.json) are used by the Rust SDK to resolve and track deployed package versions.

## Related Libraries

- [Rust SDK](https://github.com/iotaledger/notarization/tree/main/notarization-rs/README.md)
- [Wasm SDK](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/README.md)
- [Repository Root](https://github.com/iotaledger/notarization/tree/main/README.md)

## Contributing

We would love to have you help us with the development of IOTA Notarization. Each and every contribution is greatly valued.

Please review the [contribution](https://docs.iota.org/developer/iota-notarization/contribute) sections in the [IOTA Docs Portal](https://docs.iota.org/developer/iota-notarization/).

To contribute directly to the repository, simply fork the project, push your changes to your fork and create a pull request to get them included.

The best place to get involved in discussions about this package or to look for support at is the `#notarization` channel on the [IOTA Discord](https://discord.gg/iota-builders). You can also ask questions on our [Stack Exchange](https://iota.stackexchange.com/).
