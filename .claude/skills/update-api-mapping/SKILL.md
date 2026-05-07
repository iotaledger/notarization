---
name: update-api-mapping
description: Update a product's `api_mapping.toml` by diffing the working tree against a user-provided git revision
    and reconciling added/removed public Move functions/structs and their Rust and WASM/TS counterparts.
---

# Update a product's `api_mapping.toml` from a git diff

## Purpose

For an IOTA Trust Framework product (e.g. Audit Trail, Notarization, …) the
`api_mapping.toml` lists every public Move function and struct in the product's
Move sources together with the Rust entities and WASM/TS entities that wrap or
correspond to them.

When the product API changes — new Move functions, renamed Rust types, removed
WASM bindings — this file must be kept in sync. This skill takes a user-supplied
**base revision** plus three product paths and updates the TOML to reflect what
changed between that revision and the current working tree.

## Required inputs

The caller (user or invoking agent) **must** provide all of the following. If any
are missing, ask once with a concrete suggestion before proceeding. Do not pick
defaults silently.

1. **`rust-crate-path`** — path to the `src` folder of the product's Rust
   implementation (e.g. `audit-trail-rs/src`, `notarization-rs/src`).
2. **`wasm-bindings-path`** — path to the `src` folder of the product's WASM
   bindings (typically `bindings/wasm/<product-name>_wasm/src`).
3. **`move-sc-path`** — path to the `sources` folder of the product's Move
   smart contracts (e.g. `audit-trail-move/sources`, `notarization-move/sources`).
4. **base revision** — a git ref (commit SHA, branch, tag, or `HEAD~N`) to diff
   against. Suggest `origin/main` if the user offers no hint.

Try to guess the correct values for the `rust-crate-path`, `wasm-bindings-path`
and `move-sc-path` depending on the folder the user currently works in or if the
user mentions the product name. Present the input argument values to the user for
validation.

### Derived paths

- **`api-mapping-path`** — `<move-sc-path>/../api_mapping.toml`. The TOML lives
  in the parent of the Move sources directory (i.e. the Move package root). If
  it doesn't exist there, ask the user where it lives before continuing.
- **`wasm-docs-path`** — `<wasm-bindings-path>/../docs/` for generated WASM/TS
  documentation. Always treated as read-only / ignore for diffing.

## TOML key convention

Each TOML section key has the form `<product>.<module>.<entity>`:

- `<product>` — the product identifier, derived from the basename of the Move
  package directory (the parent of `move-sc-path`) with any trailing `-move`
  stripped and `-` replaced by `_`. Examples:
  - `audit-trail-move/sources` → product `audit_trail`
  - `notarization-move/sources` → product `notarization`
- `<module>` — `main` for the Move source file whose basename matches the
  product name (e.g. `audit_trail.move` → `main`, `notarization.move` →
  `main`); for any other Move source file, the bare filename without
  extension (e.g. `locking.move` → `locking`).
- `<entity>` — the function name or struct/enum/const name within that module.

If the existing TOML uses a different convention, treat the existing file as
authoritative and follow its conventions; do not rename keys.

## Scope of changes considered

Only diffs inside these paths are relevant:

- `<move-sc-path>/` — drives which TOML keys exist
- `<rust-crate-path>/` — drives the `rust` arrays
- `<wasm-bindings-path>/` — drives the `wasm` arrays

Ignore changes in tests, examples, docs, generated WASM output (`wasm-docs-path`),
Move build artifacts (`build/`, `Move.lock`), and anything else outside the
three paths above.

## What counts as a "new" or "obsolete" item

**New** — present at `HEAD` (working tree) but not in the base revision:

- Move: `public fun <name>`, `public struct <Name>`, `public enum <Name>`,
  `const <NAME>` (only if convention in the existing TOML lists consts).
  `public(package) fun <name>` is **excluded** — only fully `public` items
  enter the mapping.
- Rust: any `pub fn`, `pub struct`, `pub enum`, `impl` method exposed via
  `pub`, or `pub` field that is plausibly a wrapper for a Move entity.
- WASM: any `#[wasm_bindgen]`-exposed type, method, or getter, or any
  `Wasm*` struct/enum and its public methods.

**Obsolete** — present in the base revision but no longer in `HEAD`:

- A whole TOML section becomes obsolete if its Move function/struct was
  removed (or renamed without the TOML being updated). Remove the section
  entirely.
- An individual entry inside a `rust` or `wasm` array becomes obsolete if
  the symbol it names no longer exists in the corresponding source tree.
  Remove just that entry.

Renames are handled as remove-then-add. If you can pair an obsolete name
with a new name (same surrounding context, same signature), surface the
pairing to the user before applying — it's usually a rename and the TOML
update is mechanical.

## Workflow

1. **Validate inputs.** Confirm all three paths exist and resolve the base
   revision:

   ```bash
   git rev-parse --verify <user-ref>
   ```

   Stop and ask if anything doesn't resolve.

2. **Get the scoped diff.** Run:

   ```bash
   git diff <base>..HEAD -- \
     <move-sc-path>/ \
     <rust-crate-path>/ \
     <wasm-bindings-path>/
   ```

   Also check the working tree for uncommitted changes to those paths
   (`git status --short`) so unstaged additions are not missed. If any are
   present, include them in the analysis and tell the user.

3. **Extract added/removed Move entities.** From the diff in `<move-sc-path>/`,
   collect:
   - Added lines matching `public fun <name>`, `public struct <Name>`,
     `public enum <Name>`.
   - Removed lines matching the same patterns.
   - Ignore `public(package)` — those are not part of the mapping.
   - Ignore signature-only changes (argument types, return types) when the
     name is unchanged; those don't move TOML keys.

4. **Extract added/removed Rust entities.** From the diff in `<rust-crate-path>/`,
   collect added/removed `pub fn`, `pub struct`, `pub enum`, methods inside
   `impl` blocks declared `pub`, and `pub` struct fields. Convert to the TOML
   form: `Type::method` for inherent methods, `Type` for plain types,
   `Type::Variant` for enum variants, `Type.field` for fields,
   `module::function` for free functions.

5. **Extract added/removed WASM entities.** Same analysis applied to
   `<wasm-bindings-path>/`. Convention: WASM entities are typically prefixed
   `Wasm*`. Include only items annotated with `#[wasm_bindgen]` (directly or
   via an enclosing impl block).

6. **Reconcile against the existing TOML.** Read `<api-mapping-path>` and
   produce three lists:
   - **TOML keys to add** — Move entities newly present in `HEAD` that have
     no key.
   - **TOML keys to remove** — keys whose Move entity no longer exists.
   - **Array entries to add/remove** — Rust/WASM symbols added or removed
     under existing keys.

   For each new TOML key, propose an initial `rust` and `wasm` array by
   matching the new Move name against the new Rust/WASM symbols. Use these
   heuristics:
   - Same name (snake_case → snake_case for fns; PascalCase → PascalCase
     for types; PascalCase ↔ `Wasm` + PascalCase for WASM types).
   - For a new Move `public fun foo`, look for a Rust `Foo` transaction
     struct with `Foo::new`, a builder method, and a WASM `WasmFoo` with
     `build_programmable_transaction` / `apply_with_events` — that's the
     established pattern (see existing TOML entries for examples).
   - Leave arrays empty (`[]`) when no plausible match exists; flag for
     user input rather than guessing.

7. **Show the proposed changes before editing.** Present a concise diff:
   - Sections to add (with proposed `rust`/`wasm` arrays).
   - Sections to remove.
   - Per-section additions/removals to `rust`/`wasm` arrays.

   Wait for user confirmation if any item required heuristic matching or
   if obsolete items might be renames. Skip the confirmation gate for
   purely mechanical updates (e.g. an obvious added entry whose Rust/WASM
   counterparts have identical names to other entries already in the file).

8. **Apply the edits.** Use `Edit` to modify `<api-mapping-path>` in place.
   Preserve:
   - The file header comment.
   - The `# =====` module banner comments.
   - The order: sections grouped by module, in source file order; entries
     within `rust`/`wasm` arrays roughly follow the order in the source
     file (declaration order).

9. **Verify.** After editing:
   - Re-read the TOML and confirm it parses (every section has a key in
     the form `<product>.<module>.<entity>`, every value is an array of
     strings).
   - For each Rust/WASM entry in modified sections, confirm the symbol
     actually exists in the corresponding source tree (`grep -n "fn <name>"`
     or `grep -n "struct <Name>"` in the relevant directory).
   - Report what was changed and any items left as `[]` for the user to
     fill in.

## Operating rules

- **Never invent symbols.** If a Rust or WASM name is not found in the
  source tree, do not add it to the TOML. Either find the real name or
  flag the gap.
- **Preserve human curation.** Existing `rust`/`wasm` arrays may include
  related types beyond direct wrappers (e.g. an event struct alongside a
  transaction struct). Do not prune entries just because their name
  doesn't match the Move name — only remove entries whose underlying
  symbol no longer exists.
- **Don't reorder unrelated sections.** Limit edits to the sections being
  added, removed, or modified. The TOML is human-edited and arbitrary
  diffs are noisy in code review.
- **One product per invocation.** This skill operates on a single product
  (one `(rust-crate-path, wasm-bindings-path, move-sc-path)` triple). To
  update mappings for several products, invoke the skill once per product.
- **Stop on ambiguity.** If a Move function clearly maps to multiple
  candidate Rust types, ask the user rather than picking one.
- **Diff hygiene.** When showing the proposed change set, group by Move
  module so a reviewer can scan it the same way the file is laid out.

## Example invocation

### Example 1

> `/update-api-mapping notarization base=653a27c`

1. Explore the child folder names in the repository root folder and in the `bindings/wasm` folder
   and figure out if the three input arguments can be guessed from the product name `notarization`.
2. Present the guessed input arguments to the user for validation or - in case of doubts - ask the
   user for the correct product name or input arguments.
4. Diff that 653a27c..HEAD restricted to the three source paths.
5. Find e.g. an added `public fun foo_bar` in `notarization.move`, an
   added `FooBar` struct + `FooBar::new` in `notarization-rs/src`,
   and `WasmFooBar` with the usual two methods in the WASM crate.
6. Propose a new `[notarization.main.foo_bar]` section with both arrays
   prefilled.
7. Find e.g. that `delete_foo`'s removed Rust helper
   `FooBar::delete_legacy` should be dropped from the existing
   section.
8. Show the change set, apply on confirmation, verify, summarize.

### Example 2

User:

> `/update-api-mapping`
> `rust-crate-path=audit-trail-rs/src`
> `wasm-bindings-path=bindings/wasm/audit_trail_wasm/src`
> `move-sc-path=audit-trail-move/sources`
> base=`origin/feat/audit-trails-dev`

Expected behavior:

1. Resolve `origin/feat/audit-trails-dev` to a SHA.
2. Diff that SHA..HEAD restricted to the three source paths.
3. Find e.g. an added `public fun pause_trail` in `audit_trail.move`, an
   added `PauseTrail` struct + `PauseTrail::new` in `audit-trail-rs/src`,
   and `WasmPauseTrail` with the usual two methods in the WASM crate.
4. Propose a new `[audit_trail.main.pause_trail]` section with both arrays
   prefilled.
5. Find e.g. that `delete_records_batch`'s removed Rust helper
   `TrailRecords::delete_batch_legacy` should be dropped from the existing
   section.
6. Show the change set, apply on confirmation, verify, summarize.