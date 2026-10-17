# Experimental UCR JSON Mapping

This document describes the JSON mapping implemented by `seruq-ucr-json` for the
current Rust model. It is a draft, not a stable storage format or a complete mapping
of every feature proposed in the [UCR v0 specification](README.md).
Actions, colors, and Geometry drawing data are not implemented yet. There is no
JSON Schema at this stage.

## Document Structure

A document is a UTF-8 JSON object with three required fields:

| Field | JSON representation |
| --- | --- |
| `metadata` | Object with a required string `version` and optional string `default_font` |
| `root` | Element ID string |
| `elements` | Object mapping Element ID strings to Element objects |

The codec preserves `metadata.version` as an opaque string. It does not currently
reject unknown UCR versions, infer migrations, or equate this value with the Rust
crate version. Version compatibility checks remain to be defined.

Each Element object has three required fields:

| Field | JSON representation |
| --- | --- |
| `position` | Object with numeric `x` and `y` |
| `size` | Object with numeric `width` and `height` |
| `kind` | Object identified by a required string `type` |

The `type` field selects the following attributes inside `kind`:

| Type | Required fields | Optional fields |
| --- | --- | --- |
| `container` | `children`: array of Element ID strings | None |
| `text` | `text`: string, `font_size`: Float | `font`: string |
| `image` | `source`: string | None |
| `button` | `label`: string, `font_size`: Float | `font`: string |
| `text_input` | `value`: string, `font_size`: Float | `font`: string, `placeholder`: string |
| `geometry` | None | None |

Optional fields may be omitted or set to `null`; both decode as absent. Writers
omit absent fields. Required fields must be present, including empty `children`
arrays. A root may be any Element type.

For example:

```json
{
  "metadata": {
    "version": "0.1.0",
    "default_font": "fonts/default.ttf"
  },
  "root": "0",
  "elements": {
    "0": {
      "position": { "x": 0, "y": 0 },
      "size": { "width": 320, "height": 120 },
      "kind": { "type": "container", "children": ["1"] }
    },
    "1": {
      "position": { "x": 16, "y": 16 },
      "size": { "width": 288, "height": 32 },
      "kind": { "type": "text", "text": "Hello, Seruq", "font_size": 24 }
    }
  }
}
```

## IDs

Every ID, including `root`, children, and `elements` keys, is an unsigned 64-bit
integer encoded as a decimal string. For example, `"18446744073709551615"` is valid
and retains its exact value in languages whose ordinary JSON numbers cannot
represent every u64 integer exactly.

The syntax is `"0"` or a nonzero decimal digit followed by zero or more decimal
digits, with a maximum value of 18446744073709551615. Leading zeros, signs,
whitespace, exponents, and numeric JSON values are rejected.

Duplicate `elements` keys are rejected rather than silently overwritten.
Writers sort entries by their numeric ID to produce stable output. Object order
has no UI meaning; `children` retains its original order.

## Floating-Point Values

Finite f32 values use JSON numbers. The decoder rounds numbers to f32 and rejects
numbers that overflow to infinity. Negative zero is preserved by the Rust codec.

Non-finite values use these exact JSON strings:

| Value | JSON representation |
| --- | --- |
| NaN | `"NaN"` |
| Positive infinity | `"inf"` |
| Negative infinity | `"-inf"` |

Other string spellings and finite numeric strings such as `"16"` are rejected.
NaN payload bits and the sign of a NaN are not preserved.

These representations allow a draft to retain invalid font sizes for debugging,
including unused elements whose font sizes are not subject to UI validation.
They do not relax the constraints of Point or Size.

## Decoding and Validation

Decoding returns `UcrDraft`; it does not run `validate()` or `build()` implicitly.
Missing references, cycles, duplicate child references, and invalid font sizes can
be decoded and inspected. Use the core validator to check the reachable UI.

Point and Size values are constructed through their validating constructors:

- `position.x` and `position.y` must be finite; negative values are allowed.
- `size.width` and `size.height` must be finite and nonnegative; zero is allowed.

These type constraints apply even to unused elements. Construction failures report
the Element ID, whether `position` or `size` failed, and the geometry error reason.
Other representation errors, including duplicate IDs, are reported as JSON errors.

Unknown fields, unknown Element types, duplicate known fields, missing required
fields, and trailing data are rejected. Strings must decode to valid Unicode;
unpaired surrogate escapes are rejected.

Serialization preserves the draft's elements and references without requiring a
valid UI structure. It does not resolve fonts or images, run FFR calls, or perform
migration. Saving a validated UCR uses the same mapping through `ucr.data()`.
