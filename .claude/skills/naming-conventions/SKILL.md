---
name: naming-conventions
description: Audit README files and public-entity doc comments across Move, Rust and WASM/TS sources for compliance with the `Naming Conventions` section of the repository's root `CLAUDE.md`, and optionally apply fixes.
---

# Naming Conventions audit & fix

## Purpose

The repository's root `CLAUDE.md` defines a `Naming Conventions` section that
governs how the **Notarization Toolkit**, its TF products (**Single
Notarization**, **Audit Trail**, …), Notarization Methods, and per-language
**Packages** must be referred to in prose. The conventions distinguish product
names (title case, singular) from instance plurals (lowercase) and forbid
synonyms like "SDK" or "Suite" as package labels.

This skill reads the current `Naming Conventions` section and uses it as the
**single source of truth** to:

1. Audit README files and the doc comments of public entities in the Move,
   Rust, and WASM/TS source trees.
2. Report violations with file:line and a proposed replacement.
3. Optionally apply the fixes.

The root `CLAUDE.md` `Naming Conventions` section is authoritative — if the
section changes, this skill picks up the new rules automatically. Do not
hard-code rule snapshots inside this skill.

## When to invoke this skill

- The user says "audit the naming", "check naming conventions", "fix naming",
  "make sure we use Audit Trail correctly", or similar.
- The user just edited the `Naming Conventions` section of `CLAUDE.md` and
  wants the codebase brought into alignment.
- The user is preparing a release or doc PR and wants a sweep.

## Inputs

The skill is **self-scoping**: it walks the repo from the root and applies
the conventions to README files and public-entity prose. Two optional inputs:

1. **`scope`** — one of:
   - `all` (default) — audit every README and every public-entity doc
     comment across Move, Rust, and WASM/TS source trees in the repo.
   - `readmes` — audit only README files.
   - `sources` — audit only public-entity doc comments in source files.
   - A path or glob (e.g. `audit-trail-rs/src`, `bindings/wasm/**`) —
     audit only that subtree.
2. **`mode`** — `audit` (default; report only) or `fix` (apply edits, with
   user confirmation for anything beyond mechanical case changes).

If the user names a single TF product ("audit the audit-trail naming",
"naming sweep for notarization"), narrow scope accordingly:

- "audit trail" → `audit-trail-move/`, `audit-trail-rs/`,
  `bindings/wasm/audit_trail_wasm/`, plus any README at the repo root that
  references it.
- "notarization" / "single notarization" → `notarization-move/`,
  `notarization-rs/`, `bindings/wasm/notarization_wasm/`, plus the repo-root
  README.

## Workflow

1. **Read the source of truth.** Open the root `CLAUDE.md` and extract the
   `## Naming Conventions` section verbatim. Use those rules — and only
   those — as the basis for findings. Do not import rules from prior runs
   or from memory.

2. **Build the in-scope file list.** Per `scope`:
   - READMEs: every `README.md` not under `node_modules/`, `target/`,
     `build/`, or `.git/`.
   - Source prose: doc comments on public entities only. Public means:
     - **Move** — `///` and `/** … */` on `public fun`, `public struct`,
       `public enum`, `const`, and module-level `///` at the top of a
       `.move` file. Skip `public(package)` and private functions.
     - **Rust** — `///` and `//!` on `pub fn`, `pub struct`, `pub enum`,
       `pub` fields, `pub` consts, `pub mod` declarations, and crate-level
       `//!` in `lib.rs`/`mod.rs`. Skip `pub(crate)`, `pub(super)`,
       `pub(in …)`, and private items.
     - **WASM** — `///` on items annotated `#[wasm_bindgen]` (or inside a
       `#[wasm_bindgen]`-annotated impl block), plus crate-level `//!`.

3. **Audit each in-scope file.** Apply the `Naming Conventions` rules
   you extracted in step 1 — they are the only rules. This step adds
   only the auditor-specific judgement that the rules themselves don't
   spell out:

   **Product-sense vs instance-sense — the central judgement call.**
   Most product names in the rules (`Audit Trail`, `Notarization`, …)
   are required to be title case *when they refer to the TF product
   itself*, but the same words may appear lowercase in plural or
   instance form (per the rules' own examples). Deciding which sense
   an occurrence is in requires reading the surrounding sentence, not
   a regex. Lean on these heuristics:

   - **Product-sense indicators** (title case expected): the word is
     acting as a proper noun; replace it with the literal product name
     ("Audit Trail") and the sentence still reads correctly; it labels
     a package, client family, event family, smart-contract bundle, or
     enum surface for the product; it sits in a module-level or
     crate-level doc describing the crate as a whole.
   - **Instance-sense indicators** (lowercase permitted): the word is
     preceded by an article or quantifier ("a", "an", "the", "each",
     "one", "this"); it refers to one on-chain object or to many; it
     appears in an error message or runtime string about a specific
     object; replacing it with "this object" reads sensibly.

   When the sentence is genuinely ambiguous, prefer `INCONSISTENT` and
   flag for confirmation rather than rewriting.

   **Identifier exemption.** Code identifiers — struct/enum/module/
   function names, `iota_notarization::notarization`, dependency names
   (`iota-sdk`, `@iota/sdk`), error-variant names, Cargo.toml fields,
   JSON package names — are not prose and are out of scope. Only
   comments and Markdown narrative are eligible for findings.

   **Generated-artifact exemption.** Files under `bindings/wasm/*/docs/**`
   are generated from Rust `#[wasm_bindgen]` doc attributes. Never edit
   them directly — fix the Rust source attribute, and re-generation
   will propagate.

4. **Classify each finding.** Per occurrence, emit one of:
   - `VIOLATION` — clear breach (e.g. `Audit Trails` as product name,
     `Notarization SDK`, lowercase "audit trail" used in a product-sense
     sentence).
   - `INCONSISTENT` — permitted by the rules but stylistically out of step
     with siblings in the same file (e.g. a list of `Move Package`,
     `Rust Package`, `wasm bindings` where the third entry should match
     the labelling style of the first two).
   - `OK` — compliant; do not report.

5. **Report (audit mode) or fix (fix mode).**

   - In **audit mode**, group findings by file in source order, quote the
     offending sentence with `file:line`, and show the proposed
     replacement. End with a one-paragraph summary: total files scanned,
     total `VIOLATION`s, total `INCONSISTENT`s, and which TF
     product(s)/Package(s) each cluster of findings affects.

   - In **fix mode**, apply mechanical edits without asking (pure
     case-change replacements within the same word, e.g.
     `audit trail clients` → `Audit Trail clients` in a comment that
     unambiguously refers to the product). For anything that requires
     rephrasing (e.g. swapping `the bindings` for `the Wasm Package`,
     rewording a section heading), present the proposed change and wait
     for confirmation. Group edits by file.

6. **Verify after editing.** When fix mode touched Rust or WASM
   source files, run:
   - `cargo check -p <rust-crate>` for any modified Rust crate.
   - `cargo check --target wasm32-unknown-unknown` from the WASM
     bindings crate root for any modified WASM crate.
   - For Markdown-only changes no compile check is needed.

7. **Summarize.** Print total findings, fixes applied, fixes deferred for
   user confirmation, and any items the skill chose to skip (with the
   reason).

## Operating rules

- **Source of truth is `CLAUDE.md`.** Re-read it at the start of each run.
  Do not rely on rule snapshots stored in this file, in memory, or in
  prior turns.
- **Product-sense vs instance-sense requires reading the sentence.**
  This is not a regex job. A blind `s/audit trail/Audit Trail/g` will
  damage correct lowercase instance-plurals like "audit trails on the
  IOTA ledger". When uncertain, treat the occurrence as `INCONSISTENT`
  and flag for confirmation rather than rewriting.
- **Public-entity scope only for source prose.** Private items, doc
  comments on `pub(crate)`/`pub(super)` items, and inline `//` comments
  are not in scope. Test files and example crates are not in scope
  unless the user names them explicitly.
- **Identifiers are exempt.** Never rewrite `iota_notarization::notarization`,
  struct names, module names, dependency names, error variant names,
  Cargo.toml fields, package names in JSON, etc. Only prose comments and
  Markdown narrative text are eligible.
- **Don't touch generated artifacts.** Skip `bindings/wasm/*/docs/**`,
  `target/**`, `build/**`, `node_modules/**`. Generated `.d.ts` files
  reflect Rust doc attributes; fix the Rust attribute instead.
- **One repo per invocation.** This skill operates on the current repo.
- **Pair with sync-product-docs.** After a naming fix that touches doc
  comments, the result is still a single-layer edit; if the Move/Rust/WASM
  triplet's other layers need the same wording change, follow up with
  `sync-product-docs` rather than expanding this skill's scope.

## What "compliant" looks like — examples

**OK — instance-sense or carve-out applies:**

- `/// Creates a new audit trail with an optional initial record.`
  — "a new audit trail" is an instance.
- `/// Top-level locking configuration for the audit trail.`
  — "the audit trail" reads as the current instance.
- `A client for creating and managing audit trails on the IOTA blockchain.`
  — instance plural, lowercase per rule example.
- `The toolkit includes:` (backref to "IOTA Notarization Toolkit")
  — standard capitalization permitted for backrefs.
- `Build the wasm bindings yourself if you have Rust installed.`
  — carve-out applies: refers to the binding code.

**VIOLATION — clear breach of the rules:**

- `//! Package management for audit trail smart contracts.`
  — "audit trail" here labels the product → `Audit Trail`.
- `/// Permission variants enumerated by the audit trail.`
  — refers to the product enum surface → `Audit Trail`.
- `## I want a toolkit to build an application`
  — uses `toolkit` as a label for what is a Package.
- `Notarization SDK`
  — forbidden synonym.
- `Audit Trails` (as product name)
  — forbidden plural-as-product.

**INCONSISTENT — permitted but stylistically off:**

- `- **wasm bindings** for JavaScript and TypeScript integrations`,
  sitting in a list of `Move Package`, `Rust Package`, …
  — allowed under the carve-out but out of step with the sibling labels.

## Example invocations

### Example 1

> `/naming-conventions`

Expected behavior:

1. Read `## Naming Conventions` from the root `CLAUDE.md`.
2. Walk all READMEs and all public-entity doc comments in
   `audit-trail-move/`, `audit-trail-rs/`, `notarization-move/`,
   `notarization-rs/`, `bindings/wasm/*/`.
3. Print a per-file findings list with `VIOLATION` / `INCONSISTENT` and
   proposed replacements.
4. Print a summary.

### Example 2

> `/naming-conventions audit-trail fix`

Expected behavior:

1. Narrow scope to `audit-trail-move/`, `audit-trail-rs/`,
   `bindings/wasm/audit_trail_wasm/`, and any repo-root README that
   references Audit Trail.
2. Apply mechanical case-change fixes without asking. For anything that
   needs rephrasing (e.g. section heading rewrites, swapping
   `the bindings` for `the Wasm Package`), present the proposed change
   and wait for confirmation.
3. After editing Rust/WASM sources, run the relevant `cargo check`
   commands.
4. Summarize.

### Example 3

> `/naming-conventions readmes`

Expected behavior:

1. Audit only README files (root and per-package).
2. Report; do not edit unless `mode=fix` is also specified.
