# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IOTA Notarization enables creation of immutable, on-chain records for arbitrary data by storing it (or a hash) in dedicated Move objects on the IOTA ledger. The workspace has two main subsystems: **Notarization** (creating tamper-proof records) and **Audit Trails** (structured, role-based audit logging).

## Common Commands

### Build & Check
```bash
cargo build --workspace --tests --examples
cargo check -p notarization-rs
cargo check -p audit-trail-rs
```

### Test
```bash
# Tests must run single-threaded (IOTA sandbox requirement)
cargo test --workspace --release -- --test-threads=1

# Single test
cargo test --release -p notarization-rs test_name -- --test-threads=1

# Move contract tests (from notarization-move/ or audit-trail-move/)
iota move test
```

### Lint & Format
```bash
cargo clippy --all-targets --all-features
cargo fmt --all
cargo fmt --all -- --check   # check only
```

### WASM Bindings (in bindings/wasm/notarization_wasm/ or audit_trail_wasm/)
```bash
npm install
npm run build
npm test           # Node.js tests
npm run test:browser  # Cypress browser tests
```

### Move Scripts
```bash
# From notarization-move/ or audit-trail-move/
./scripts/publish_package.sh
./scripts/notarize.sh
```

### Running Examples
Examples require the notarization package to be published first. From the repo root:
```bash
# Publish the package and capture the package ID
export IOTA_NOTARIZATION_PKG_ID=$(./notarization-move/scripts/publish_package.sh)

# Run a specific example
cargo run --release --example <example_name_goes_here>
```
To run all examples. From the repo root::
```bash
# Make sure IOTA_NOTARIZATION_PKG_ID is set as shown above
./examples/run.sh 
```

## Workspace Structure

The root `Cargo.toml` defines a workspace with members: `notarization-rs`, `audit-trail-rs`, `examples`. The WASM crates (`bindings/wasm/*`) are excluded from the workspace and built separately.

- **`notarization-rs/`** — Rust client library for notarization
- **`notarization-move/`** — Move smart contracts for notarization
- **`audit-trail-rs/`** — Rust client library for audit trails
- **`audit-trail-move/`** — Move smart contracts for audit trails
- **`bindings/wasm/notarization_wasm/`** — JS/TS WASM bindings for notarization
- **`bindings/wasm/audit_trail_wasm/`** — JS/TS WASM bindings for audit trails
- **`examples/`** — Rust examples (basic CRUD + real-world scenarios like IoT, legal contracts)

## Architecture

### Client Layer Pattern
Both `notarization-rs` and `audit-trail-rs` follow the same pattern:
- **Full client** (`NotarizationClient` / `AuditTrailClient`): Signs and submits transactions
- **Read-only client** (`NotarizationClientReadOnly` / `AuditTrailClientReadOnly`): Read-only state inspection
- Clients wrap a `product_common` transaction builder that supports `.build()`, `.build_and_execute()`, and `.execute_with_gas_station()`

### Builder Pattern (Type-State)
Notarization creation uses a `NotarizationBuilder<T>` with phantom type states to enforce valid configurations at compile time. Separate builder paths exist for **Dynamic** (mutable, transferable) vs **Locked** (immutable, non-transferable) notarizations.

### Method Types
- **Dynamic**: State and metadata are updatable after creation; supports transfer locks
- **Locked**: State and metadata are immutable; supports time-based destruction

### Lock System
- **Transfer locks**: `None`, `UnlockAt(epoch)`, `UntilDestroyed`
- **Delete locks**: Restrict when a notarization can be destroyed

### Cross-Platform Compilation
Code uses `#[cfg(target_arch = "wasm32")]` guards to conditionally compile for WASM. Features `send-sync`, `gas-station`, `default-http-client`, and `irl` control optional capabilities.

### Key External Dependencies
- `iota-sdk` (v1.19.1, from IOTA git) — on-chain interaction
- `iota_interaction` / `iota_interaction_rust` / `iota_interaction_ts` — from `product-core` repo, `feat/tf-compoenents-dev` branch
- `product_common` — transaction builder abstraction from `product-core`
- `secret-storage` (v0.3.0) — key management

## Testing Requirements

- Tests require an IOTA sandbox running locally
- Always use `--test-threads=1` (tests share sandbox state)
- Examples require `IOTA_NOTARIZATION_PKG_ID` environment variable set to the deployed package ID
- WASM browser tests use Cypress

## Rust Version

Minimum: **1.85**, Edition: **2024**
