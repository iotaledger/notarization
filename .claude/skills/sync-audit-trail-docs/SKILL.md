---
name: sync-audit-trail-docs
description: Sync the doc comments of public Move functions/structs in audit-trail-move with the corresponding Rust (audit-trail-rs) and WASM/TypeScript (bindings/wasm/audit_trail_wasm) entities, using audit-trail-move/api_mapping.toml as the source of truth for the mapping.
---

# Sync audit trail documentation across Move, Rust and WASM layers

## Purpose

The audit trail subsystem has three implementation layers that need to convey
the same behavior to their users:

- **Move** — `audit-trail-move/sources/*.move`
- **Rust** — `audit-trail-rs/src/**/*.rs`
- **WASM/TypeScript** — `bindings/wasm/audit_trail_wasm/src/**/*.rs`

`audit-trail-move/api_mapping.toml` is the canonical mapping from each public
Move function/struct to the Rust and WASM entities that wrap, build, or
otherwise correspond to it. This skill uses that mapping to keep the doc
comments of each "triplet" (Move ↔ Rust ↔ WASM) semantically aligned.

The Move layer is the **authoritative source of behavior**. Its doc comments
describe what the on-chain function does, what arguments it takes, what events
it emits, and when it aborts. Rust and WASM doc comments must convey the same
contract, framed for the language they live in.

## When to invoke this skill

- The user says "sync the audit trail docs", "check the api_mapping doc
  alignment", "update the Rust/WASM docs to match Move", or similar.
- The user has just edited a `.move` file in `audit-trail-move/sources/` and
  asks to propagate the doc change.
- The user wants a report of where the three layers disagree.

Do **not** invoke this skill for the notarization subsystem — it covers only
the audit trail crates listed above.

## Inputs and scope

Always read `audit-trail-move/api_mapping.toml` first. It is the source of
truth for which entities are paired up. Do not invent additional pairings or
skip entries the file lists.

Each TOML key has the form `<move_module>.<move_entity>`:

- `<move_module>` is one of `audit_trail.main`, `audit_trail.locking`,
  `audit_trail.permission`, `audit_trail.record`, `audit_trail.record_tags`.
  These map to files in `audit-trail-move/sources/` as follows:
  - `audit_trail.main` → `audit_trail.move`
  - `audit_trail.locking` → `locking.move`
  - `audit_trail.permission` → `permission.move`
  - `audit_trail.record` → `record.move`
  - `audit_trail.record_tags` → `record_tags.move`
- `<move_entity>` is the function name or struct/enum name within that module.

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

1. **Parse `audit-trail-move/api_mapping.toml`.** Build the list of triplets
   (Move entity → Rust entries → WASM entries). If the user named a specific
   module/entity, filter to that subset.

2. **For each Move entity, locate its doc comment.** In Move, doc comments are
   `///` lines (or `/** ... */`) immediately preceding the `public fun`,
   `public struct`, `public enum`, or `const` declaration. If the entity is a
   struct/enum, also collect the per-field/per-variant `///` comments where
   applicable.

3. **For each `rust` entry, locate the doc comment in `audit-trail-rs/src/`.**
   Use grep to find the declaration (`fn <name>`, `struct <Type>`,
   `enum <Type>`, `impl ... { fn <name> ... }`, etc.). Doc comments are `///`
   lines preceding the item. For `Type.field`, find the field inside the
   struct definition.

4. **For each `wasm` entry, do the same in
   `bindings/wasm/audit_trail_wasm/src/`.** WASM types are typically prefixed
   with `Wasm` and bound via `#[wasm_bindgen]`. Some WASM entries are exposed
   as TypeScript via tsify/jsdoc — check that the rendered TS doc (visible in
   `bindings/wasm/audit_trail_wasm/docs/` or in the `#[wasm_bindgen(...)]`
   attribute) matches.

5. **Compare semantically, not character-by-character.** A Rust doc may
   reasonably:
   - Rephrase to fit Rust idioms (e.g. "Returns `Option<T>`" instead of Move's
     "returns the value if set").
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
   - **Permission requirements** in `audit_trail.permission.*` → the Rust
     `Permission::*` and WASM `WasmPermission::*` variants must describe the
     same gated operation.
   - **Locking semantics** in `audit_trail.locking.*` → time-based vs
     count-based windows, and the `UntilDestroyed` constraint on
     `delete_trail_lock`, must be consistent.
   - **Field-level docs** for structs like `LockingConfig`, `Record`,
     `ImmutableMetadata` — Move documents fields with `///`; the Rust struct
     fields and WASM getter accessors must match.

6. **Report (audit mode) or edit (fix mode).** For each triplet, report one
   of:
   - `OK` — semantically aligned, no action.
   - `MISSING <layer>` — the entity exists per the mapping but has no doc
     comment in that layer.
   - `DRIFT <layer>` — the doc exists but contradicts or omits a contract
     point from the Move source. Quote the diverging sentence.
   - `MAPPING STALE` — an entry in the TOML refers to a Rust/WASM symbol that
     does not exist in the source tree. Suggest fixing the TOML rather than
     the docs.

   In fix mode, propose the new Rust/WASM doc comment and apply the edit with
   the `Edit` tool. Never edit the Move doc to match Rust/WASM — Move is the
   source of truth for behavior. If you believe the Move doc is wrong, flag
   it for the user instead of changing it.

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
- **Preserve existing doc style.** Match the surrounding crate's tone:
  `audit-trail-rs` uses full sentences with backticked identifiers; the WASM
  crate often uses shorter TS-friendly summaries. Don't homogenize the style
  — only align the meaning.
- **List Move Events.** If Move events are documented with the related Move
  function, also list these events at the back of Rust and TS function
  documentation. 
- **Don't touch generated artifacts.** Files under
  `bindings/wasm/audit_trail_wasm/docs/wasm/` are generated; fix the source
  Rust attributes instead.
- **Respect the project's "no gratuitous comments" rule** (see `CLAUDE.md`).
  If a Move function has no doc comment because its name fully describes it
  (e.g. simple field getters like `creator`, `created_at`), don't manufacture
  one for the Rust/WASM side — leave the triplet as-is and report `OK`.

## What "aligned" looks like — example

For `[audit_trail.locking.new]` (the `LockingConfig::new` constructor):

- **Move** (`locking.move`): "Create a new locking configuration. Aborts with
  `EUntilDestroyedNotSupportedForDeleteTrail` when `delete_trail_lock` is
  `TimeLock::UntilDestroyed`; that variant is reserved for the write lock."
- **Rust** (`LockingConfig`, `LockingConfig::to_ptb`): the type-level doc
  should mention that `delete_trail_lock` cannot be `TimeLock::UntilDestroyed`
  and explain the resulting on-chain abort the wrapper surfaces as an error.
- **WASM** (`WasmLockingConfig::new`): the constructor doc must surface the
  same constraint, framed as a JS exception that callers will see.

If the Rust doc says only "Construct a `LockingConfig`" without mentioning the
constraint, that is a `DRIFT Rust` finding.
