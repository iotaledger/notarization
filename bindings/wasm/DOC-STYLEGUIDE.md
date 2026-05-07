# DOC-STYLEGUIDE.md — Documentation Style Guide for wasm-bindgen generated TSDoc/JSDoc

This file is the authoritative style guide for the doc comments on the wasm-bindgen
bindings of IOTA Trust Framework products (TF-product), typically located
in `bindings/wasm/<PRODUCT-NAME>_wasm/src/` in the products git repository.
The Rust doc comments here are shipped to TypeScript consumers verbatim — they end up
in the generated `.d.ts` files and drive IDE IntelliSense and TypeDoc output.
They must therefore read as TSDoc/JSDoc, **not** as rustdoc.

## Audience

The reader is a TypeScript developer using the TF-product TS/JS package. They:

- Do **not** know Rust, `wasm_bindgen`, or that the bindings are generated.
- See identifiers in their TypeScript form (camelCase methods, JS class names from
  `js_name = …`).
- Browse via IDE hover, the generated `.d.ts`, and TypeDoc HTML.

Write for that audience. Never mention `wasm_bindgen`, `wasm-pack`, "wasm
consumers", "JS/TS bindings", "exposed to JavaScript", or anything that betrays
the binding mechanism. Describe behavior from the TypeScript user's perspective.

## What gets documented

Every **exported binding surface** must have a description:

- Each `#[wasm_bindgen]` struct or enum.
- Each `pub` method/function inside a `#[wasm_bindgen]` `impl` block.
- Each public field of a `#[wasm_bindgen(getter_with_clone)]` struct.
- Each variant of a `#[wasm_bindgen]` enum.
- Each `pub` parameter of every documented function (use `@param`).

Local helpers (private `fn`s, `pub(crate)` items, `From`/`TryFrom` impls, panic
hooks, internal closures) do **not** need TSDoc. Module-level `//!` comments are
also internal and are not exported.

## Linking

Use TSDoc/JSDoc link syntax only. Never use Rust intra-doc links.

Examples from `https://github.com/iotaledger/notarization/tree/feat/audit-trails-dev/bindings/wasm/audit_trail_wasm`

| Use                                       | Don't use                          |
|-------------------------------------------|------------------------------------|
| `{@link AuditTrailBuilder}`               | `` [`AuditTrailBuilder`] ``        |
| `{@link AuditTrailBuilder.withAdmin}`     | `` [`Self::with_admin`] ``         |
| `{@link RoleMap.initialAdminRoleName \| Admin}` (display text) | rust path-style links |
| `{@link Permission.AddRecord}`            | enum path links                    |

Always link by **TS-visible name** (the value of `js_name`, or the camelCase form
that wasm-bindgen produces), not by Rust identifier:

Examples:

- `WasmAuditTrailBuilder` → link as `AuditTrailBuilder`.
- `with_admin` → link as `withAdmin`.
- A field `valid_from_ms` → link as `validFromMs`.

Do not produce broken links. If the symbol is not exported, refer to it in
backticks (`` `addRecord` ``) instead of via `{@link …}`.

## Allowed tags

Use only TSDoc tags (the ones standard TypeDoc understands). The set in use:

- `@remarks` — extended description after the summary.
- `@param name - description.` — one per parameter, hyphen-prefixed.
- `@returns description.` — return value (see omission rule below).
- `@throws description.` — error conditions.
- `@inheritDoc {@link Other.member}` — when copy-inheriting docs.

Do **not** use rustdoc-only tags (`# Errors`, `# Examples`, `# Safety`,
`# Panics`) — they render as literal headings in TS output.

## Block ordering and spacing

Every doc comment that has more than a summary line follows this order, with a
blank line (`///` line on its own) between each block:

```
/// Summary line — one short sentence.
///
/// @remarks
/// Extended description, multiple sentences, possibly multiple paragraphs.
///
/// Requires the {@link Permission.<TS-Permission-Name-Goes-Here>} permission.
///
/// @param foo - Description of foo.
/// @param bar - Description of bar.
///
/// @returns Description of the return value.
///
/// @throws Description of error conditions.
///
/// Emits a {@link <TS-EventType-Goes-Here>} event on success.
```

Rules:

- Summary first, on its own line(s), no tag.
- `@remarks` block comes next when present.
- The "Requires …" capability block comes after `@remarks` (or directly after
  the summary when no `@remarks` exists). Always its own paragraph, separated
  by blank lines on both sides. The "Requires …" capability block is only
  needed if the TF-products Move smart contract is access controlled e.g.
  by Capability objects or equivalent access control mechanism. See below
  for more details.
- `@param` lines are grouped together with no blank lines between them, then a
  blank line before the next block.
- `@returns` follows `@param`s.
- `@throws` follows `@returns`.
- Move-event lines come **last**, separated from the preceding block by a blank
  line. They are written as plain prose ("Emits a {@link Foo} event on
  success."), not as a tag.
- Multi-paragraph `@remarks` separate paragraphs with `///` blank lines inside
  the same `@remarks` block.

## Capability gating

When a function/transaction is gated on one or more capabilities/permissions,
state it in a dedicated paragraph that begins with "Requires …" and link the
permission(s) (examples are using 
[RoleMap based access control](https://github.com/iotaledger/product-core/blob/main/components_move/sources/role_map.move)):

```
/// Requires the {@link Permission.<TS-Permission-Name-Goes-Here>} permission.
```

For multiple permissions, list them naturally:

```
/// Requires the {@link Permission.AddFoo} and {@link Permission.AddBar}
/// permissions.
```

The block is separated from surrounding prose by blank lines on both sides.

When the function is not access gated, omit the block entirely.

## Move events

Functions whose Move counterpart emits an event end their doc with one
event-emission paragraph per event, separated from the preceding content by a
blank line:

```
/// Emits a {@link <TS-Event-Name-Goes-Here>} event on success.
```

For multiple emissions:

```
/// Emits one {@link FooBarDeleted} event per deletion.
```

When the function emits no event, omit the block entirely. Do not write "Emits
no event."

## Parameter docs

- Every parameter visible to TypeScript callers gets an `@param` line.
- Use the **TS-visible name**: `@param sequenceNumber` not `@param sequence_number`.
- Use `name - description.` form (TSDoc's hyphen form). End with a period.
- Optional parameters: describe both the `null`/`undefined` semantics and the
  set semantics. Example:
  `@param message - Optional message shown displayed to the user.`

If a parameter is purely structural (e.g. takes a builder and returns one),
still document it — at minimum: "Configured `Foo`."

## Return docs

- Document non-trivial return values with `@returns`.
- **Omit** `@returns` when the underlying Rust function returns `Result<(), …>`
  (i.e. `Ok(())`). The `Ok(())` exists only so the JS side can throw on error —
  there is no TS-visible return value.
- For builder chain methods that return `Self`, write something like:
  "@returns The same builder, with the X configured."
- For transaction-wrapper-producing methods, write:
  "@returns A {@link TransactionBuilder} wrapping the {@link Foo} transaction."

## Throws docs

Use `@throws` when the function can fail with a TS-visible exception:

- Object ID parsing failures.
- "Read-only client" guard failures.
- Network/serialization errors.
- Logical preconditions that the TF-product library rejects before submission.

Briefly state the trigger condition. One `@throws` per distinct failure
category, or a single `@throws` summarizing them when they share a phrase.

## Field docs

Public fields of `getter_with_clone` annotated structs are visible to TS as plain
properties. Document each with a one-line description above the field. If the
semantics are non-trivial (e.g. nullable, normalized, sorted), state that in
the description:

```rust
/// Sequence number of the first entry, if any.
pub head: Option<u64>,
```

For longer field documentation, the same blank-line block ordering applies.

## Enum variant docs

Each variant of an exported enum gets a one-line description:

```rust
pub enum WasmPermission {
    /// Authorizes deleting the foo-bar itself.
    DeleteFooBar,
    /// Authorizes the batched foo-bar-deletion entry point.
    DeleteAllFooBar,
    …
}
```

Don't repeat the enum's own summary on every variant; describe what the variant
does.

## Phrasing conventions

- Prefer present tense, active voice: "Returns …", "Builds …", "Replaces …".
- "On success" / "on-chain" / "aborts on-chain" are the standard phrasings for
  describing Move-side outcomes.
- Time fields: be explicit about units and epoch — "milliseconds since the Unix
  epoch", "seconds since the Unix epoch", "inclusive" / "exclusive" where it
  matters.
- Never claim something is "internal" or "for advanced users only" without a
  concrete reason.
- Do not document the Rust-side type — document the TypeScript-side behavior.
  E.g. write "Returns a string view of the payload." not "Calls
  `String::from_utf8_lossy`."

## Forbidden content

- Mentions of `wasm_bindgen`, `wasm-pack`, "wasm exports", "wasm consumers",
  "JS/TS bindings", "JS-friendly wrapper", "WASM-friendly".
- Rust intra-doc link syntax (``[`Foo`]``, ``[`Foo`](path)``).
- Section headings (`# Errors`, `# Examples`) — they don't render in TSDoc.
- Markdown emoji unless the user explicitly asks for it.
- "Returns Ok(())" or "Returns unit" — instead, omit `@returns`.
- Implementation details that are not observable from TypeScript.

## Validating changes

After changing doc comments:

1. `cargo check --target wasm32-unknown-unknown` — must succeed.
2. If the crate has `#![warn(rustdoc::all)]`:
   `cargo doc --target wasm32-unknown-unknown --no-deps` — should produce no
   `rustdoc::all` warnings ().
3. If feasible, regenerate the `.d.ts` (`npm run build`) and skim the output
   for any reference that looks broken or contains a stray rustdoc artifact.

## Quick template

When adding a new exported function, start from this template and trim any
block that does not apply:

```rust
/// <One-sentence summary.>
///
/// @remarks
/// <Optional extended description.>
///
/// Requires the {@link Permission.<Foo>} permission.
///
/// @param <name> - <Description>.
///
/// @returns <Description, or omit for `Ok(())` returns.>
///
/// @throws <Description.>
///
/// Emits a {@link <FooEvent>} event on success.
#[wasm_bindgen(js_name = <jsName>)]
pub fn <rust_name>(...) -> Result<...> { ... }
```
