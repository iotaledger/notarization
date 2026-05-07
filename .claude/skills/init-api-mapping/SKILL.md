---
name: init-api-mapping
description: Create an initial `api_mapping.toml` for a new IOTA Trust Framework product, then delegate to the `update-api-mapping` skill to populate it with all currently public Move/Rust/WASM entities.
---

# Bootstrap a product's `api_mapping.toml`

## Purpose

A new IOTA Trust Framework product (or a product whose Move/Rust/WASM code
already exists but has never had an API mapping committed) needs a starting
`api_mapping.toml` placed in the Move package root. This skill scaffolds that
file with the standard header and conventions, then hands off to the
`update-api-mapping` skill to fill in every section by diffing against the
empty tree — i.e. treating *every* current public Move/Rust/WASM entity as
"newly added".

The actual extraction, naming, and reconciliation logic lives in
`update-api-mapping`. This skill exists only to:

1. Establish the file at the right path with the right header.
2. Invoke `update-api-mapping` with the right inputs so the bootstrap pass
   reconciles "nothing" against "everything currently in `HEAD`".
3. Hand a populated, verified TOML back to the user.

## When to invoke this skill

- The user says "create an api_mapping for `<product>`", "bootstrap the api
  mapping", "we need an `api_mapping.toml` for the new `<product>` crate",
  or similar.
- A product directory has Move/Rust/WASM code but no `api_mapping.toml`
  alongside the Move package.
- The user explicitly invokes `/init-api-mapping`.

Do **not** invoke this skill when an `api_mapping.toml` already exists at
the target location — use `update-api-mapping` instead. If one exists and
the user really wants to recreate it, confirm first that they want the
existing file overwritten before proceeding.

## Required inputs

The caller (user or invoking agent) **must** provide all of the following.
If any are missing, try to guess them from the user's current working
directory or from the product name they mentioned, and present the guesses
back for validation. Do not pick defaults silently.

1. **`rust-crate-path`** — path to the `src` folder of the product's Rust
   implementation (e.g. `audit-trail-rs/src`, `notarization-rs/src`).
2. **`wasm-bindings-path`** — path to the `src` folder of the product's
   WASM bindings (typically `bindings/wasm/<product-name>_wasm/src`).
3. **`move-sc-path`** — path to the `sources` folder of the product's Move
   smart contracts (e.g. `audit-trail-move/sources`).

Try to guess the correct values for the `rust-crate-path`, `wasm-bindings-path`
and `move-sc-path` depending on the folder the user currently works in or if the
user mentions the product name. Present the input argument values to the user for
validation.

### Derived values

- **`api-mapping-path`** — `<move-sc-path>/../api_mapping.toml`. This is
  where the new file will be created.
- **`product`** — the product identifier, derived from the basename of
  the Move package directory (parent of `move-sc-path`) with any trailing
  `-move` stripped and `-` replaced by `_`. Examples:
  - `audit-trail-move/sources` → `audit_trail`
  - `notarization-move/sources` → `notarization`
- **`product-display`** — a human-readable form for the file's title
  comment (e.g. `Audit Trail`, `Notarization`). Derive by title-casing
  the package basename with `-move` stripped and `-` → space; confirm
  with the user if uncertain.

## Workflow

1. **Validate inputs.** Confirm:
   - All three paths exist.
   - `<move-sc-path>` actually contains `*.move` files.
   - `<api-mapping-path>` does **not** already exist. If it does, stop
     and tell the user to use `update-api-mapping` instead (or ask
     whether they want to overwrite — never overwrite silently).

2. **Identify the Move package's "main" file.** List `*.move` files in
   `<move-sc-path>` and identify the one whose basename matches the
   product identifier (with any underscore/dash normalisation needed).
   That file's entities will go under the `<product>.main.*` keys; all
   others use their bare filename. If no file matches the product name,
   ask the user which file should be considered "main" (or whether the
   convention should be relaxed for this product).

3. **Create the scaffold TOML.** Write `<api-mapping-path>` with this
   exact header and one banner per Move source file (in source order),
   leaving each module body empty for the next step to populate:

   ```toml
   # <product-display> API Mapping
   #
   # Maps each public Move function or struct in the `<move-sc-path>/`
   # modules to the related Rust entities in `<rust-crate-path>/` and
   # WASM/TS entities in `<wasm-bindings-path>/`.
   #
   # TOML section keys are formed as `<product>.<module>.<entity>`:
   #   - `<product>` — the product identifier, derived from the basename
   #     of the Move package directory with any trailing `-move` stripped
   #     and `-` replaced by `_`. For this file: `<product>`.
   #   - `<module>`  — `main` for the Move source file whose basename
   #     matches the product name (`<product>.move` → `main`); for any
   #     other Move source file, the bare filename without extension.
   #   - `<entity>`  — the function name or struct/enum/const name in
   #     that module.
   #
   # `rust` and `wasm` arrays list the Rust- resp. WASM-level functions,
   # methods, and types that wrap, build, or otherwise correspond to the
   # Move entity. Entry conventions:
   #   - `Type::method`     — an inherent method on `Type`
   #   - `Type::Variant`    — an enum variant
   #   - `Type`             — a plain type/struct/enum
   #   - `Type.field`       — a struct field
   #   - `module::function` — a free function
   #
   # An entry of `[]` means there is intentionally no counterpart on
   # that side.
   #
   # This mapping is intended for automatic comparison of function and
   # struct documentation across the three implementation layers, and is
   # maintained via the `update-api-mapping` and `sync-product-docs`
   # skills under `.claude/skills/`.

   # =============================================================================
   # Module: <product>::main (<move-sc-path>/<product>.move)
   # =============================================================================

   # =============================================================================
   # Module: <product>::<other_module> (<move-sc-path>/<other_module>.move)
   # =============================================================================
   ```

   Substitute the placeholders (`<product>`, `<product-display>`,
   `<move-sc-path>`, `<rust-crate-path>`, `<wasm-bindings-path>`) with
   the actual values. Emit one banner per `.move` file, in the order
   they appear in `<move-sc-path>` (alphabetical, with the `main` file
   first).

4. **Bootstrap the contents via `update-api-mapping`.** Invoke the
   `update-api-mapping` skill with:

   - `rust-crate-path`, `wasm-bindings-path`, `move-sc-path` — the same
     values supplied to this skill.
   - **base revision** = the git empty-tree SHA
     `4b825dc642cb6eb9a060e54bf8d69288fbee4904`. Diffing against this
     SHA causes every public Move/Rust/WASM entity in the working tree
     to be reported as "added", which is exactly what bootstrapping
     needs.

   `update-api-mapping` will then:

   - Detect every `public fun`, `public struct`, `public enum` in the
     Move sources and propose a section per entity.
   - Match Rust/WASM symbols using its standard naming heuristics.
   - Write all sections to the freshly scaffolded TOML.

   Follow `update-api-mapping`'s normal confirmation flow. Because this
   is a bootstrap, expect a large change set — group the proposal by
   Move module so the user can review one file at a time.

5. **Verify the result.** After `update-api-mapping` finishes:

   - Re-read `<api-mapping-path>` and confirm it parses.
   - Confirm every Move public entity in `<move-sc-path>` has a
     corresponding TOML section. If any are missing, list them and ask
     the user whether to add them with `[]` arrays or pick up after
     manual investigation.
   - Confirm every banner has at least one section under it (or note
     that the module is intentionally empty).
   - Report a summary: number of sections created, number with
     non-empty `rust`/`wasm` arrays, number left as `[]` for the user
     to fill in, and any items that needed user input during the
     bootstrap.

## Operating rules

- **Don't duplicate `update-api-mapping`'s logic.** This skill scaffolds
  and delegates; it must not extract symbols, propose Rust/WASM matches,
  or edit individual sections itself. If `update-api-mapping`'s behavior
  needs to change, change *that* skill, not this one.
- **One product per invocation.** Bootstrap one mapping at a time so
  the bootstrap diff is reviewable.
- **Never overwrite an existing TOML silently.** Stop and ask if
  `<api-mapping-path>` already exists.
- **Don't invent Move source files.** Only create banners for `*.move`
  files actually present in `<move-sc-path>`.
- **Preserve the empty-tree contract.** The hand-off to
  `update-api-mapping` always uses the empty-tree SHA as the base.
  Don't substitute `origin/main` or similar — those would compare
  against an unrelated history and miss entities.

## Example invocation

User:

> `/init-api-mapping`
> `rust-crate-path=identity-rs/src`
> `wasm-bindings-path=bindings/wasm/identity_wasm/src`
> `move-sc-path=identity-move/sources`

Expected behavior:

1. Confirm none of the three paths is missing and that
   `identity-move/api_mapping.toml` does not yet exist.
2. Inspect `identity-move/sources/` — find e.g. `identity.move`,
   `credentials.move`, `revocation.move`. Identify `identity.move` as
   the "main" file (matches product name `identity`).
3. Write the scaffold TOML at `identity-move/api_mapping.toml` with
   header and three module banners (`identity::main`,
   `identity::credentials`, `identity::revocation`).
4. Invoke `update-api-mapping` with base
   `4b825dc642cb6eb9a060e54bf8d69288fbee4904`. It detects every
   `public fun`/`public struct`/`public enum` in those three Move
   files, proposes Rust and WASM counterparts via its naming
   heuristics, and (after user confirmation) fills the file in.
5. Re-read the resulting TOML, summarise: e.g. "47 sections created;
   42 have non-empty `rust` arrays; 5 left as `[]` for review:
   `identity.main.rotate_keys`, …".