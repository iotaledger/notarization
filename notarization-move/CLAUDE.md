# CLAUDE.md — Move guidelines for `notarization-move`

## Documentation Style Guide

Follow the guidelines in `../MOVE-DOC-STYLEGUIDE.md` and make sure to
follow all rules stated there.

## No Capability based access control

Ignore `Capability gating` related rules in `../MOVE-DOC-STYLEGUIDE.md` because
`notarization_wasm` only uses access control through Move object ownership.

## Notarization-product-specific terminology and rules

### Notarization Methods

`Dynamic` and `Locked` are the **Notarization Methods**. Always refer to them
as Notarization Methods (or, where unambiguous, simply `method`). Do **not**
use synonyms such as "variant", "kind", "type", "behavioural variant",
"flavour", "mode", "sort", or similar. When mentioning a specific method, use
the bare enum name in backticks (`` `Dynamic` ``, `` `Locked` ``) — the full
path `NotarizationMethod::Dynamic` is not needed in prose. The compound terms
"Dynamic-Notarization" and "Locked-Notarization" refer to a `Notarization`
configured with the corresponding method.

Use generic Notarization Method based descriptions if suitable. Do not reduce
the usage of Notarization Methods to the currently available variants
(like `... is dynamic or locked`) because in future versions there may be more
Notarization Method variants. Only explain a behavior using specific Notarization
Method variants if a function (or other item) is explicitly focussed (or limited)
to one or more specific Notarization Methods.

### Method-dependent behavior must be a bullet list

Whenever the behavior of a documented entity (function, struct, field, enum,
event, constant) differs between Notarization Methods, document the
differences as a Markdown bullet list with one bullet per method, in this
fixed order:

```move
/// ...
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: <what happens for a Dynamic-Notarization>.
/// * `Locked`: <what happens for a Locked-Notarization>.
```

This format must be kept even when the rule for one method is trivial
("Always returns `false`.") so that future Notarization Methods can be added
as additional bullets without restructuring the surrounding prose. Never
collapse the per-method behavior into a single sentence such as "mutable
only for the dynamic method".