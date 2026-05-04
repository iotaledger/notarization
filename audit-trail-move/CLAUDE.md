# CLAUDE.md — Move documentation style guide (`audit-trail-move`)

This file is the canonical style guide for `///` doc comments on every Move
item in `audit-trail-move/sources/`. Apply it whenever you add or edit a doc
comment in this package.

The goal is that a reader of the on-chain Move source — and of the Rust and
WASM/TS bindings generated from it (see `api_mapping.toml`) — can understand
each function's contract without having to read the function body or chase
across modules.

## Doc-comment syntax

- Use `///` line comments only. Do not use `/** ... */`.
- Place the doc comment on the lines immediately preceding the item it
  documents, with no blank line in between.
- Use Markdown inside doc comments; doc-tools render it.
- Wrap lines at roughly 100 columns. Continuation lines of a Markdown bullet
  are indented two spaces under the `*`.

## Function doc structure

Every public function (`public fun` and `public(package) fun`) and every
`entry fun` follows this structure. Each section is a separate paragraph
separated from neighbours by an empty `///` line.

1. **Summary sentence** (mandatory) — a single short sentence describing what
   the function does. Use present tense, third-person ("Creates …",
   "Returns …", "Checks whether …"). Do not begin with the function name.
2. **Behaviour paragraphs** (optional) — one or more paragraphs describing
   internal behaviour, preconditions, postconditions, invariants, and
   cross-references to related functions. Include only what a caller needs
   beyond the summary.
3. **`Requires …` paragraph** — required when the function gates on a
   capability/permission. Phrase as "Requires a capability granting the
   `<Permission>` permission." When multiple roles or conditions apply,
   list them in one sentence or a short bullet list.
4. **`Aborts with:` paragraph** — required when the function can abort.
   Format as a Markdown bullet list of every abort cause (see next
   section).
5. **`Emits …` paragraph** — required when the function emits one or more
   events. Phrase as "Emits a `<Event>` event on success." For multiple
   events, "Emits `<EventA>` and `<EventB>` …" or one bullet per event.
6. **`Returns …` paragraph** — required when the function has a non-trivial
   return. For trivial getters whose summary already says "Returns the X."
   the explicit `Returns …` paragraph may be omitted.

The order is fixed: summary → behaviour → Requires → Aborts → Emits →
Returns. Only include the sections that apply.

### Trivial getters and constructors

A function whose summary already conveys everything (e.g. a one-line getter
returning a stored field, or a one-line constructor wrapping an enum
variant) does not need any further sections. Keep it as a single short
sentence.

```move
/// Returns the address that created this trail.
public fun creator<D: store + copy>(self: &AuditTrail<D>): address { ... }
```

## `Aborts with:` formatting

List every abort cause as a Markdown bullet. The intro line is
`/// Aborts with:` on its own. Each bullet is `/// * <cause>.` and ends with
a full stop. Continuation lines of a bullet are indented two spaces under
the `*`.

```move
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordTagAlreadyDefined` when `tag` is already in the registry.
```

Conventions inside bullets:

- Identify the abort by the **error constant name** in backticks
  (`` `EFoo` ``). For external errors use the fully qualified path
  (`` `tf_components::capability::EValidityPeriodInconsistent` ``).
- Use lower-case prose for the condition: ``* `EFoo` when ...``.
- For aborts that bubble up from a delegated call, reference the
  authoritative abort list rather than re-listing every variant. Example:
  ``* any error documented by `RoleMap::assert_capability_valid` when `cap`
  fails authorization checks.``
- Order bullets from most general to most specific: package-version checks
  first, then capability/permission validation, then function-specific
  causes.

## `Requires …` formatting

A single sentence in the active voice referring to the capability:

```move
/// Requires a capability granting the `AddRecord` permission.
```

When the function additionally requires role-tag-allowlist authorization or
similar, append it to the same sentence:

```move
/// Requires a capability granting the `AddRecord` permission and, when
/// `record_tag` is set, a role whose `RoleTags` allow that tag.
```

## `Emits …` formatting

```move
/// Emits a `RecordAdded` event on success.
```

For batch operations:

```move
/// Emits one `RecordDeleted` event per deletion.
```

For multiple distinct events use a bullet list with the same convention as
`Aborts with:`.

## `Returns …` formatting

For functions whose summary does not already describe the result:

```move
/// Returns the constructed `LockingConfig`.
```

For tuples, name the components:

```move
/// Returns the tuple `(admin_cap, trail_id)`: the initial admin `Capability`
/// and the ID of the newly shared `AuditTrail` object.
```

For `Option<T>` returns, document both branches:

```move
/// Returns `option::some(bytes)` when `data` is `Data::Bytes`, otherwise
/// `option::none()`.
```

## Cross-references and identifiers

- Refer to types, functions, fields, and constants in backticks. Use
  `Type::method` for methods, `Type::Variant` for enum variants,
  `Type.field` for fields, and `module::function` for free functions.
- When a wrapper delegates to a function in another module, link by name
  (`` `RoleMap::assert_capability_valid` ``) instead of duplicating its
  abort list. The reader can follow the reference; we don't drift out of
  sync.
- Inside the same module, omit the module prefix
  (`` `add_record` `` rather than `` `audit_trail::main::add_record` ``).
- Refer to permission variants by their bare enum name in backticks
  (`` `AddRecord` `` rather than `` `Permission::AddRecord` ``) — the
  context makes the type unambiguous and matches the permission constants
  emitted by helper constructors.
- Units must be explicit when stating timestamps: "milliseconds since the
  Unix epoch" or "seconds". Do not write "ms" or "s" as a bare suffix.

## Tone and wording

- Present tense, third-person, active voice.
- Begin with a verb: "Creates …", "Returns …", "Removes …", "Checks
  whether …".
- Avoid hedging ("usually", "tries to", "may"). State invariants directly.
- Do not write the same fact twice. If the summary already carries the
  information, do not repeat it under `Returns`.
- Do not document what is obvious from a short, well-named function — a
  trivial getter does not need an `Aborts` section saying it cannot abort.

## Struct, enum, constant, and event docs

- Document each public struct, enum, and event with a short summary
  sentence above the definition.
- Document each public field of a struct with a `///` line above the field.
  Field docs follow the same brevity rules as function summaries.
- Error constants (`#[error] const E…`) carry the user-facing abort message;
  no separate doc comment is required when the message is self-explanatory.
- Module-level docs (the `///` block above `module audit_trail::…;`) must
  describe the module's purpose in one or two sentences.

## Don'ts

- Don't add `Parameters:` / `Arguments:` sections — Move parameter names
  serve as their own documentation; describe their meaning in the relevant
  paragraph instead.
- Don't add `Notes:` or `Warning:` headings — write a behaviour paragraph
  instead. Genuine warnings about destructive or irreversible operations
  may use an inline `WARNING:` prefix on a paragraph.
- Don't quote the abort message string. Reference the error constant name.
- Don't number bullet points unless ordering is meaningful.
- Don't add a doc comment that simply restates the function name in
  English ("Get the trail creator address"). Either add value or omit.

## Worked example

```move
/// Adds a record to the trail at the next available sequence number.
///
/// Records are appended sequentially with auto-assigned sequence numbers.
/// When `record_tag` is set, the trail's tag-registry usage count for that
/// tag is incremented.
///
/// Requires a capability granting the `AddRecord` permission and, when
/// `record_tag` is set, a role whose `RoleTags` allow that tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ETrailWriteLocked` while `write_lock` is active.
/// * `ERecordTagNotDefined` when `record_tag` is not in the trail's tag registry.
/// * `ERecordTagNotAllowed` when `cap`'s role does not allow `record_tag`.
///
/// Emits a `RecordAdded` event on success.
public fun add_record<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    record_tag: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) { ... }
```

## When in doubt

The Move sources are the source of truth for the audit-trail product's
behaviour. Doc comments must reflect the on-chain contract precisely:
arguments, return semantics, abort/error conditions, emitted events,
authorization requirements, and locking/timing constraints.

If the code and the doc disagree, fix the doc — and verify the discrepancy
is not also present in the Rust and WASM/TS layers (see the
`sync-audit-trail-docs` skill).
