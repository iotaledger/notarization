# Skill examples

Practical examples for the three project-local skills under `.claude/skills/`.
Each skill is invoked by typing its slash-command in a Claude Code session
(e.g. `/init-api-mapping`) or by asking in natural language — the descriptions
below show both.

The three skills work together:

```
init-api-mapping  ─►  update-api-mapping  ─►  sync-product-docs
   (bootstrap)         (reconcile API)         (propagate docs)
```

Inputs every skill needs (it will try to guess from your cwd, but always
validate the guesses):

- `move-sc-path`    — e.g. `notarization-move/sources`
- `rust-crate-path` — e.g. `notarization-rs/src`
- `wasm-bindings-path` — e.g. `bindings/wasm/notarization_wasm/src`

---

## 1. `init-api-mapping` — bootstrap a new product's mapping

Creates `<move-sc-path>/../api_mapping.toml` with the standard header, then
delegates to `update-api-mapping` to populate every section from `HEAD`.

**When to use:** the product has Move/Rust/WASM code but no `api_mapping.toml` yet.

### Example prompts

```text
/init-api-mapping
```

```text
Bootstrap the api_mapping for a new product called "credentials".
Its Move sources are in credentials-move/sources, the Rust crate is
credentials-rs/src, and the WASM bindings live in
bindings/wasm/credentials_wasm/src.
```

```text
We have a new TF product under audit-log-move/. Please create its
api_mapping.toml.
```

### Expected outcome

- `<product-package>/api_mapping.toml` exists with the canonical header
  comment and one section per public Move entity, each with its Rust and
  WASM/TS counterparts pre-populated.
- The skill refuses (rather than overwriting) if the file already exists.

---

## 2. `update-api-mapping` — reconcile after API changes

Diffs the working tree against a base revision and updates the `rust` /
`wasm` arrays of each entry. Adds new sections, removes entries for deleted
Move entities, and reports anything ambiguous.

**When to use:** public Move/Rust/WASM API changed since the last mapping update.

### Example prompts

```text
/update-api-mapping
```

```text
Update the notarization api_mapping against origin/main.
```

```text
I just renamed two Move functions in audit-trail-move/sources/record.move.
Please reconcile audit-trail-move/api_mapping.toml against HEAD~5.
```

```text
A few public WASM bindings were removed. Update the notarization
api_mapping.toml using origin/feat/audit-trails-dev as the base.
```

### Expected outcome

- New public Move entities appear as new `[<product>.<module>.<entity>]`
  sections, with `rust = [...]` / `wasm = [...]` populated where matches
  exist and `[]` where they intentionally don't.
- Entries for removed Move entities are dropped.
- A short report lists ambiguous matches that need human judgment.

---

## 3. `sync-product-docs` — propagate Move docs to Rust and WASM/TS

Walks `api_mapping.toml` and aligns the doc comments of each (Move, Rust,
WASM) triplet. The Move layer is authoritative; Rust and WASM/TS docs are
edited to convey the same contract in their respective styles (rustdoc /
TSDoc).

**When to use:** Move doc comments changed and the Rust/WASM/TS layers
need to be brought back in line — or you want a drift report.

### Example prompts

```text
/sync-product-docs
```

```text
I just rewrote the doc comments on notarization-move/sources/notarization.move.
Please sync notarization-rs and notarization_wasm to match, using
notarization-move/api_mapping.toml.
```

```text
Check the audit-trail docs for drift against the Move sources — report
where the three layers disagree but don't edit anything yet.
```

```text
Sync the notarization doc comments end-to-end (Move → Rust → WASM/TS),
following the styleguides referenced from notarization-move/CLAUDE.md and
bindings/wasm/notarization_wasm/CLAUDE.md.
```

### Expected outcome

- Rust doc comments (rustdoc) and WASM doc comments (TSDoc) are updated
  to mirror the Move semantics for every entity listed in the mapping.
- Per-method bullet lists, abort/error sections, and `Emits …` lines are
  rendered in the form prescribed by `MOVE-DOC-STYLEGUIDE.md` and
  `bindings/wasm/DOC-STYLEGUIDE.md`.
- `cargo check`, `cargo doc`, and `cargo check --target wasm32-unknown-unknown`
  all still pass.

---

## Typical end-to-end workflow

```text
# 1. Move API changed on a branch
/update-api-mapping        # base revision: origin/main

# 2. After Move doc comments are rewritten
/sync-product-docs         # propagates Move docs into Rust + WASM/TS
```

For a brand-new product:

```text
/init-api-mapping          # bootstrap the mapping (delegates to update-api-mapping)
/sync-product-docs         # initial pass across the three layers
```
