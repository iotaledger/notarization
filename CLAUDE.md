# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IOTA Notarization enables creation of immutable, on-chain records for arbitrary data by storing it (or a hash) in dedicated Move objects on the IOTA ledger. The workspace has two components: **Notarization** (creating tamper-proof records) and **Audit Trail** (structured, role-based audit logging).

## Naming Conventions

- Everything contained in this repository is part of the **Notarization Toolkit** - do not use synonyms like "Notarization Suite",
  "Notarization SDK" or similar labels for the Notarization Toolkit.
- The Notarization Toolkit is part of the IOTA Trust Framework
  - Use standard capitalization for the word `Toolkit` (includes "title case" where needed) - use "Toolkit" or "toolkit"
    whatever suites into the context the best. In this stylguide `Toolkit` is used for referencing the term. Use "title case"
    allways for `Notarization Toolkit` (never use `Notarization toolkit` or `notarization toolkit`).
- The IOTA Trust Framework consist of Trust Framework Products (TF products)
- The Notarization Toolkit contains two TF products: **Single Notarization** and **Audit Trail**
  - In the context of Notarization Toolkit documentation, Single Notarization and Audit Trail are called components
  - In the context of IOTA Trust Framework documentation, Single Notarization and Audit Trail are called TF products
  - These rules also apply to future TF products in the Notarization Toolkit (i.e. "Proof of Inclusion")
- The name of TF products resp. Notarization Toolkit components is allways a singular term
  - Use capitalization (a.k.a. title case) for the words of a product name if the product is meant itself - examples `Audit Trail`, `Notarization`
  - Use plural (i.e. `audit trails` or `notarizations`) only where multiple instances of the TF product are meant
    - Use lower case for the plural form - except at the beginning of sentences and in markdown titles
  - In situations where the TF product itself or the plural form can be addressed choose whatever fits best
  - This rule - including capitalization aspects - only applies to TF products resp. Notarization Toolkit components using
    plural for other entities like i.e. Notarization Methods (`Locked Notarization`, `Dynamic Notarization` - see below) is OK.
  - Example `Audit Trail`:
    - Do not use `Audit Trails` - always use `Audit Trail` to denote the product itself
    - Use plural (i.e. `audit trails` or `Audit trails`) only where multiple instances of the TF product are meant - Examples:
      - `A client for creating and managing audit trails on the IOTA blockchain`
      - `Audit trails and their records are ...`
      - `Audit trails provide ...` (could also be `Audit Trail provides ...`)
- Regarding Single Notarization (Component/TF product):
  - Single Notarization provides two **Notarization Methods**: **Locked Notarization** and **Dynamic Notarization**
    - There might be additional Notarization Methods in future versions of Single Notarization (i.e. "Custom Notarization")
    - For Notarization Methods, the following can be used to describe or identify the method (whatever suites into the context the best):
      - Short Name: `Locked`, `Dynamic`, ...
      - Full Name: `Locked Notarization`, `Dynamic Notarization`, ...
- Each TF product/component provides packages for Move, Rust and WASM/TypeScript:
  - Do not use terms like `toolkit`, `SDK`, ... for the packages - only use the term `Package`
  - Use standard capitalization for the word `Package` (includes "title case" where needed) - use "Package" or "package"
    whatever suites into the context the best. In this stylguide `Package` is used for referencing the term.
  - Aspects regarding the use of `Package` for software development in general:
    - For Move the term `Package` is allways used
    - In Rust contexts, the term `Package` denotes a bundle of one or more crates containing a Cargo. toml file. Use the term
      `Crate` and `Package` whatever suites into the context the best
    - For WASM:
      - The term `Package` can have two meanings:
        - The WASM-Rust package containing the WASM binding code
          -  If the documentation refers to this aspect (i.e. explaining the existence of wasm bindings in the Rust package)
             the term "wasm bindings" instead of `Package` is OK.
        - The JS/TS package created out of the WASM-Rust binding code using wasm-bindgen
      - In most contexts this doesn't need to be distinguished, so just use the term `Package`

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

Examples require the relevant Move package to be published first.

**Notarization examples** — from the repo root:

```bash
# Publish the package and capture the package ID
export IOTA_NOTARIZATION_PKG_ID=$(./notarization-move/scripts/publish_package.sh)

# Run a specific example
cargo run --release --example <example_name_goes_here>
```

To run all notarization examples:

```bash
# Make sure IOTA_NOTARIZATION_PKG_ID is set as shown above
./examples/run.sh
```

**Audit Trail examples** — from the repo root:

```bash
# Publish the package; on localnet both vars are set to the same package ID
eval $(./audit-trail-move/scripts/publish_package.sh)

# Run a specific example
cargo run --release --example <example_name_goes_here>
```

The `eval` form is required because the publish script prints shell `export` statements for two variables:

- `IOTA_AUDIT_TRAIL_PKG_ID` — the Audit Trail package ID
- `IOTA_TF_COMPONENTS_PKG_ID` — the TfComponents package ID (equals `IOTA_AUDIT_TRAIL_PKG_ID` on localnet)

## Developing Examples

### Adding a new example

1. Create the source file under `examples/notarization/` or `examples/audit-trail/`.
2. Add an `[[example]]` entry to `examples/Cargo.toml` pointing to the new file.
3. Use `examples::get_funded_notarization_client()` (notarization) or `examples::get_funded_audit_trail_client()` (Audit Trail) from `examples/utils/utils.rs` to obtain a funded, signed client. Do not inline client construction in example files.

### Audit Trail example patterns

Reference implementation: `examples/audit-trail/01_create_audit_trail.rs`

**Client setup** — `get_funded_audit_trail_client()` reads `IOTA_AUDIT_TRAIL_PKG_ID` and `IOTA_TF_COMPONENTS_PKG_ID` from the environment and returns `AuditTrailClient<InMemSigner>`.

**Creating a trail** — use the builder returned by `client.create_trail()`:

```rust
let created = client
    .create_trail()
    .with_trail_metadata(ImmutableMetadata::new("name".into(), Some("description".into())))
    .with_updatable_metadata("mutable status string")
    .with_initial_record(InitialRecord::new(Data::text("content"), Some("metadata".into()), None))
    .finish()
    .build_and_execute(&client)
    .await?
    .output; // TrailCreated { trail_id, creator, timestamp }
```

The creator automatically receives an Admin capability object in their wallet.

**Defining a role** — use the trail handle's access API with the implicit Admin capability:

```rust
client
    .trail(trail_id)
    .access()
    .for_role("RecordAdmin")
    .create(PermissionSet::record_admin_permissions(), None)
    .build_and_execute(&client)
    .await?;
```

`PermissionSet` convenience constructors: `admin_permissions()`, `record_admin_permissions()`, `locking_admin_permissions()`, `tag_admin_permissions()`, `cap_admin_permissions()`, `metadata_admin_permissions()`.

**Issuing a capability** — mint a capability object for a role:

```rust
let cap = client
    .trail(trail_id)
    .access()
    .for_role("RecordAdmin")
    .issue_capability(CapabilityIssueOptions::default())
    .build_and_execute(&client)
    .await?
    .output; // CapabilityIssued { capability_id, target_key, role, issued_to, valid_from, valid_until }
```

Use `CapabilityIssueOptions { issued_to, valid_from_ms, valid_until_ms }` to restrict who may use the capability or set a validity window.

**Key types** (from `audit_trail::core::types`): `Data`, `InitialRecord`, `ImmutableMetadata`, `LockingConfig`, `LockingWindow`, `TimeLock`, `Permission`, `PermissionSet`, `CapabilityIssueOptions`, `RoleTags`.

### Notarization example patterns

Reference implementations: `examples/notarization/01_create_locked_notarization.rs` and `examples/notarization/02_create_dynamic_notarization.rs`.

Use `examples::get_funded_notarization_client()` to get a `NotarizationClient<InMemSigner>`. Read `audit-trail-rs/tests/e2e/` for detailed usage of every API surface.

## Workspace Structure

The root `Cargo.toml` defines a workspace with members: `notarization-rs`, `audit-trail-rs`, `examples`. The WASM crates (`bindings/wasm/*`) are excluded from the workspace and built separately.

- **`notarization-rs/`** — Rust client library for notarization
- **`notarization-move/`** — Move smart contracts for notarization
- **`audit-trail-rs/`** — Rust client library for Audit Trail
- **`audit-trail-move/`** — Move smart contracts for Audit Trail
- **`bindings/wasm/notarization_wasm/`** — JS/TS WASM bindings for notarization
- **`bindings/wasm/audit_trail_wasm/`** — JS/TS WASM bindings for Audit Trail
- **`examples/`** — Rust examples (basic CRUD + real-world scenarios like IoT, legal contracts)

When performing checks and edits always ignore the folders and files defined in the `.gitignore` file.

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
- Notarization examples require `IOTA_NOTARIZATION_PKG_ID` environment variable set to the deployed package ID
- Audit trail examples require `IOTA_AUDIT_TRAIL_PKG_ID` (and `IOTA_TF_COMPONENTS_PKG_ID` on localnet) — use `eval $(./audit-trail-move/scripts/publish_package.sh)` to set both
- WASM browser tests use Cypress

## Rust Version

Minimum: **1.85**, Edition: **2024**
