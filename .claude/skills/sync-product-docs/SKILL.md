---
name: sync-product-docs
description: Sync the doc comments of public Move functions/structs in a product's Move sources with the corresponding Rust and WASM/TypeScript entities, using the product's api_mapping.toml as the source of truth for the mapping.
---

# Sync product documentation across Move, Rust and WASM layers

## Purpose

An IOTA Trust Framework product (Audit Trail, Notarization, …) has three
implementation layers that need to convey the same behavior to their users:

- **Move** — `<move-sc-path>/*.move`
- **Rust** — `<rust-crate-path>/**/*.rs`
- **WASM/TypeScript** — `<wasm-bindings-path>/**/*.rs`

The product's `api_mapping.toml` (located at `<move-sc-path>/../api_mapping.toml`
- see **`api-mapping-path`** below for more details)
is the canonical mapping from each public Move function/struct to the Rust and
WASM entities that wrap, build, or otherwise correspond to it. This skill uses
that mapping to keep the doc comments of each "triplet" (Move ↔ Rust ↔ WASM)
semantically aligned.

The Move layer is the **authoritative source of behavior**. Its doc comments
describe what the on-chain function does, what arguments it takes, what events
it emits, and when it aborts. Rust and WASM doc comments must convey the same
contract, framed for the language they live in.

## Required inputs

The caller (user or invoking agent) **must** provide all of the following. If any
are missing, ask once before proceeding.

1. **`rust-crate-path`** — path to the `src` folder of the product's Rust
   implementation (e.g. `audit-trail-rs/src`, `notarization-rs/src`).
2. **`wasm-bindings-path`** — path to the `src` folder of the product's WASM
   bindings (typically `bindings/wasm/<product-name>_wasm/src`).
3. **`move-sc-path`** — path to the `sources` folder of the product's Move
   smart contracts (e.g. `audit-trail-move/sources`).

Try to guess the correct values for the `rust-crate-path`, `wasm-bindings-path`
and `move-sc-path` depending on the folder the user currently works in or if the
user mentions the product name. Present the input argument values to the user for
validation.

### Derived paths

- **`api-mapping-path`** — `<move-sc-path>/../api_mapping.toml`. The TOML lives
  in the parent of the Move sources directory. If it isn't there, ask the user
  where it lives before continuing.
- **`wasm-docs-path`** — `<wasm-bindings-path>/../docs/`. Generated; never edit.

## When to invoke this skill

- The user says "sync the audit trail docs", "sync the notarization docs", "check the api_mapping doc
  alignment for notarization", "update the Rust/WASM docs to match Move", or similar.
- The user has just edited a `.move` file in a product implementation folder
  i.e. `audit-trail-move/sources/` and asks to propagate the doc change.
- The user wants a report of where the three layers disagree.

## Prepare the scope

Always read the `api_mapping.toml` file first. It is the source of
truth for which entities are paired up. Do not invent additional pairings or
skip entries the file lists.

## TOML key convention

Each TOML key has the form `<product>.<module>.<entity>`:

- `<product>` — the product identifier (derived from the Move package
  directory name with any trailing `-move` stripped and `-` replaced by `_`).
- `<module>` — `main` for the Move source file whose basename matches the
  product name; the bare filename without extension otherwise.
- `<entity>` — the function name or struct/enum/const name in that module.

The Move source file for a given module key is therefore:

- `<product>.main` → `<move-sc-path>/<product>.move` (with `_` → original
  package separator if needed)
- `<product>.<other>` → `<move-sc-path>/<other>.move`

Each entry has a `rust` and a `wasm` array. Entry conventions:

- `Type::method` — an inherent method on `Type`
- `Type::Variant` — an enum variant
- `Type` — a plain type/struct/enum
- `Type.field` — a struct field
- `module::function` — a free function

An entry of `[]` means there is intentionally no counterpart on that side.
Treat that as a legitimate state, not a missing doc.

## Workflow

Pick one of the two modes the user asks for:

**Audit mode (default when unspecified):** report mismatches without editing.
**Fix mode:** apply edits to bring Rust and/or WASM docs into alignment with
the Move doc, asking the user before non-trivial rewrites.

Follow these steps:

1. **Parse the product's api_mapping.toml** at `<api-mapping-path>`. Build the
   list of triplets (Move entity → Rust entries → WASM entries). If the user
   named a specific module/entity, filter to that subset.

2. **For each Move entity, locate its doc comment in `<move-sc-path>/`.** In
   Move, doc comments are `///` lines (or `/** ... */`) immediately preceding
   the `public fun`, `public struct`, `public enum`, or `const` declaration.
   If the entity is a struct/enum, also collect the per-field/per-variant
   `///` comments where applicable.

3. **For each `rust` entry, locate the doc comment in `<rust-crate-path>/`.**
   Use grep to find the declaration (`fn <name>`, `struct <Type>`,
   `enum <Type>`, `impl ... { fn <name> ... }`, etc.). Doc comments are `///`
   lines preceding the item. For `Type.field`, find the field inside the
   struct definition.

4. **For each `wasm` entry, do the same in `<wasm-bindings-path>/`.** WASM
   types are typically prefixed `Wasm` and bound via `#[wasm_bindgen]`. Some
   WASM entries are exposed as TypeScript via tsify/jsdoc — check that the
   rendered TS doc (visible in `<wasm-docs-path>` or in the
   `#[wasm_bindgen(...)]` attribute) matches.

5. **Compare semantically, not character-by-character.** A Rust doc may
   reasonably:
   - Rephrase to fit Rust idioms (e.g. "Returns `Option<T>`" instead of
     Move's "returns the value if set").
   - Add Rust-specific details (lifetimes, async, error type, builder
     positioning).
   - Drop on-chain-only details that don't apply at the client layer.

   A mismatch is anything that changes the **observable contract**: argument
   meaning, return semantics, abort/error conditions, emitted events,
   authorization requirements, locking/timing constraints, units (ms vs s,
   epoch vs timestamp).

   Pay particular attention to:
   - **Abort conditions in Move** → these must be reflected as documented
     errors in the Rust/WASM wrappers.
   - **Permission/authorization requirements** → wrappers on either side
     must describe the same gated operation and required capability.
   - **Locking/timing semantics** → time-based values, count-based values and
     any reserved variants (e.g. constraints valid only for specific locks)
     must be consistent across all three layers.
   - **Field-level docs** for structs that document fields with `///` —
     the Rust struct fields and WASM getter accessors must match.

6. **Report (audit mode) or edit (fix mode).** For each triplet, report one
   of:
   - `OK` — semantically aligned, no action.
   - `MISSING <layer>` — the entity exists per the mapping but has no doc
     comment in that layer.
   - `DRIFT <layer>` — the doc exists but contradicts or omits a contract
     point from the Move source. Quote the diverging sentence.
   - `MAPPING STALE` — an entry in the TOML refers to a Rust/WASM symbol
     that does not exist in the source tree. Suggest fixing the TOML
     (via the `update-api-mapping` skill) rather than the docs.

   In fix mode, propose the new Rust/WASM doc comment and apply the edit
   with the `Edit` tool. Never edit the Move doc to match Rust/WASM — Move
   is the source of truth for behavior. If you believe the Move doc is
   wrong, flag it for the user instead of changing it.

7. **Summarize at the end:** total triplets checked, OK count, mismatches by
   category, and the list of entries skipped (if any).

## Operating rules

- **Use the TOML — don't reinvent the mapping.** Even when an obvious naming
  pattern exists, only sync pairs the file declares.
- **Don't add documentation to entities the TOML lists with `[]`.** That
  empty list is intentional (no counterpart exists yet, by design).
- **Process by Move module.** Working through one `.move` file at a time
  keeps the Move source open in context and reduces churn.
- **Group edits by file.** When fixing, batch all edits to the same Rust or
  WASM file into a single pass to minimize re-reads.
- **Follow existing doc style guides.** Lookup possibly referenced documentation
  guidelines in `CLAUDE.md` files in the Rust crate or Move package folder. 
- **Preserve existing doc style.** If no documentation
  guideline can be found, match the surrounding crate's tone.
- **List Move events.** If Move events are documented with the related Move
  function, also list these events at the back of Rust and TS function
  documentation.
- **Don't touch generated artifacts.** Files under `<wasm-docs-path>/docs/**` are
  generated; fix the source Rust attributes instead.
- **One product per invocation.** This skill operates on a single product.
  To sync several products' docs, invoke the skill once per product.

## What "aligned" looks like — example

For an entry `[audit_trail.locking.new]` (the `LockingConfig::new`
constructor):

- **Move** (`<move-sc-path>/locking.move`): "Create a new locking
  configuration. Aborts with `EUntilDestroyedNotSupportedForDeleteTrail`
  when `delete_trail_lock` is `TimeLock::UntilDestroyed`; that variant is
  reserved for the write lock."
- **Rust** (`LockingConfig`, `LockingConfig::to_ptb` in `<rust-crate-path>`):
  the type-level doc should mention that `delete_trail_lock` cannot be
  `TimeLock::UntilDestroyed` and explain the resulting on-chain abort the
  wrapper surfaces as an error.
- **WASM** (`WasmLockingConfig::new` in `<wasm-bindings-path>`): the
  constructor doc must surface the same constraint, framed as a JS exception
  that callers will see.

If the Rust doc says only "Construct a `LockingConfig`" without mentioning
the constraint, that is a `DRIFT Rust` finding.

## Example invocation

### Example 1
> `/sync-product-docs notarization`

Expected behavior:

1. Explore the child folder names in the repository root folder and in the `bindings/wasm` folder
   and figure out if the three input arguments can be guessed from the product name `notarization`.
2. Present the guessed input arguments to the user for validation or - in case of doubts - ask the
   user for the correct product name or input arguments.
3. Read `notarization-move/api_mapping.toml`.
4. For each triplet, locate the Move, Rust, and WASM doc comments.
5. Report `OK` / `MISSING` / `DRIFT` / `MAPPING STALE` per triplet.
6. Edit the docs accordingly
7. Print a summary grouped by Move module.


### Example 2
User:

> `/sync-product-docs`
> `rust-crate-path=notarization-rs/src`
> `wasm-bindings-path=bindings/wasm/notarization_wasm/src`
> `move-sc-path=notarization-move/sources`
> mode=audit

Expected behavior:

1. Read `notarization-move/api_mapping.toml`.
2. For each triplet, locate the Move, Rust, and WASM doc comments.
3. Report `OK` / `MISSING` / `DRIFT` / `MAPPING STALE` per triplet.
4. Print a summary grouped by Move module.