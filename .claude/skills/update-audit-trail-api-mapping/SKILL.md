---
name: update-audit-trail-api-mapping
description: Update audit-trail-move/api_mapping.toml by diffing the working tree against a user-provided git revision and reconciling added/removed public Move functions/structs and their Rust and WASM/TS counterparts.
---

# Update `audit-trail-move/api_mapping.toml` from a git diff

## Purpose

`audit-trail-move/api_mapping.toml` lists every public Move function and
struct in `audit-trail-move/sources/` together with the Rust entities in
`audit-trail-rs/src/` and the WASM/TS entities in
`bindings/wasm/audit_trail_wasm/src/` that wrap or correspond to it.

When the audit trail API changes — new Move functions, renamed Rust types,
removed WASM bindings — this file must be kept in sync. This skill takes a
user-supplied **base revision** and updates the TOML to reflect what changed
between that revision and the current working tree.

## When to invoke this skill

- The user says "update the api_mapping", "refresh
  `audit-trail-move/api_mapping.toml`", "reconcile the api mapping with
  branch X", "what's new since commit <sha>", and similar.
- The user explicitly invokes `/update-audit-trail-api-mapping`.

## Required input

The user **must** provide a git revision to diff against (commit SHA, branch,
tag, or `HEAD~N`). If they don't, ask once with a concrete suggestion (e.g.
"`origin/main`?") before proceeding. Do not pick a default silently — the
correct base depends on what the user is reconciling.

## Scope of changes considered

Only diffs inside these paths are relevant:

- `audit-trail-move/sources/` — drives which TOML keys exist
- `audit-trail-rs/src/` — drives the `rust` arrays
- `bindings/wasm/audit_trail_wasm/src/` — drives the `wasm` arrays

Ignore changes in tests, examples, docs, generated WASM output
(`bindings/wasm/audit_trail_wasm/docs/`), Move build artifacts (`build/`,
`Move.lock`), and anything else outside the three paths above.

## What counts as a "new" or "obsolete" item

**New** — present at `HEAD` (working tree) but not in the base revision:

- Move: `public fun <name>`, `public(package) fun <name>` is **excluded**
  (only `public` enters the mapping), `public struct <Name>`,
  `public enum <Name>`. The TOML key is `<module>.<name>` where `<module>`
  is one of `audit_trail.main`, `audit_trail.locking`,
  `audit_trail.permission`, `audit_trail.record`, `audit_trail.record_tags`
  (mapping by source filename: `audit_trail.move` → `audit_trail.main`,
  others by their bare name).
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

1. **Confirm the base revision.** Resolve the user-provided ref:

   ```bash
   git rev-parse --verify <user-ref>
   ```

   Stop and ask if it doesn't resolve.

2. **Get the scoped diff.** Run:

   ```bash
   git diff <base>..HEAD -- \
     audit-trail-move/sources/ \
     audit-trail-rs/src/ \
     bindings/wasm/audit_trail_wasm/src/
   ```

   Also check the working tree for uncommitted changes to those paths
   (`git status --short`) so unstaged additions are not missed. If any are
   present, include them in the analysis and tell the user.

3. **Extract added/removed Move entities.** From the diff in
   `audit-trail-move/sources/`, collect:
   - Added lines matching `public fun <name>`, `public struct <Name>`,
     `public enum <Name>`.
   - Removed lines matching the same patterns.
   - Ignore `public(package)` — those are not part of the mapping.
   - Ignore signature-only changes (argument types, return types) when the
     name is unchanged; those don't move TOML keys.

4. **Extract added/removed Rust entities.** From the diff in
   `audit-trail-rs/src/`, collect added/removed `pub fn`, `pub struct`,
   `pub enum`, methods inside `impl` blocks declared `pub`, and `pub`
   struct fields. Convert to the TOML form: `Type::method` for inherent
   methods, `Type` for plain types, `Type::Variant` for enum variants,
   `Type.field` for fields, `module::function` for free functions.

5. **Extract added/removed WASM entities.** Same analysis applied to
   `bindings/wasm/audit_trail_wasm/src/`. Convention: WASM entities are
   prefixed `Wasm*`. Include only items annotated with `#[wasm_bindgen]`
   (directly or via an enclosing impl block).

6. **Reconcile against the existing TOML.** Read
   `audit-trail-move/api_mapping.toml` and produce three lists:
   - **TOML keys to add** — Move entities newly present in `HEAD` that
     have no key.
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
     established pattern (see `add_record`, `delete_record`, etc. in the
     existing TOML).
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

8. **Apply the edits.** Use `Edit` to modify
   `audit-trail-move/api_mapping.toml` in place. Preserve:
   - The file header comment.
   - The `# =====` module banner comments.
   - The order: sections grouped by module, in source file order; entries
     within `rust`/`wasm` arrays roughly follow the order in the source
     file (declaration order).

9. **Verify.** After editing:
   - Re-read the TOML and confirm it parses (every section has a key in
     the form `<module>.<name>`, every value is an array of strings).
   - For each Rust/WASM entry in modified sections, confirm the symbol
     actually exists in the source tree (`grep -n "fn <name>"` or
     `grep -n "struct <Name>"` in the relevant directory).
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
- **Don't touch the notarization side.** This skill only covers the audit
  trail. The notarization workspace has its own (separate) mapping if any.
- **Stop on ambiguity.** If a Move function clearly maps to multiple
  candidate Rust types, ask the user rather than picking one.
- **Diff hygiene.** When showing the proposed change set, group by Move
  module so a reviewer can scan it the same way the file is laid out.

## Example invocation

User: `/update-audit-trail-api-mapping origin/feat/audit-trails-dev`

Expected behavior:

1. Resolve `origin/feat/audit-trails-dev` to a SHA.
2. Diff that SHA..HEAD restricted to the three source paths.
3. Find e.g. an added `public fun pause_trail` in `audit_trail.move`, an
   added `PauseTrail` struct + `PauseTrail::new` in `audit-trail-rs`, and
   `WasmPauseTrail` with the usual two methods in the WASM crate.
4. Propose a new `[audit_trail.main.pause_trail]` section with both
   arrays prefilled.
5. Find e.g. that `delete_records_batch`'s removed Rust helper
   `TrailRecords::delete_batch_legacy` should be dropped from the
   existing section.
6. Show the change set, apply on confirmation, verify, summarize.