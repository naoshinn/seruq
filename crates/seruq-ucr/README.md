# seruq-ucr

The Rust model of Seruq UCR: the data types, validation, and Mutations. It has no
dependencies. Encodings, function calls, and rendering live in other crates.

## Documents

- `UcrDraft` holds `metadata`, a `root` ID, `elements`, and `state`. Its fields
  are public and can describe an invalid structure.
- `Element` holds `position`, `size`, `kind`, `actions`, and `data`.
  `Element::new` creates one without actions or data.
- `ElementKind` is Container, Text, Image, Button, TextInput, or Geometry. Only
  a Container has children, and their order is the drawing order.
- `UcrDraft::build` validates a draft and returns a `Ucr` with any warnings.
  `Ucr` is read through `data()` and changed only through `apply`.

```rust,ignore
let (mut document, warnings) = draft.build().map_err(|report| /* report.errors() */)?;
let applied = document.apply(&[MutationOp::Remove { id: 7 }])?;
```

## Validity

Constraints on values are enforced by types, which reject invalid values on
construction, so even a draft cannot contain them:

| Type | Constraint |
| --- | --- |
| `Point` | Finite |
| `Size` | Finite and nonnegative |
| `FontSize` | Finite and positive |
| `AssetPath` | Relative, `/`-separated, without `.`, `..`, `\`, `:`, or control characters |
| `Number` | Finite |

Constraints on structure are checked by `UcrDraft::validate` and `build`, which
traverse the elements reachable from the root. Every diagnostic names the
elements it concerns.

| Diagnostic | Meaning |
| --- | --- |
| `RootNotFound` | The root does not exist |
| `ElementNotFound` | A Container references a missing element |
| `RootHasParent` | A reachable Container references the root, including cycles through it |
| `Cycle` | A cycle or self-reference that does not pass through the root |
| `DuplicateReference` | A second reference to an element that already has a parent |
| `UnusedElement` (warning) | An element is not reachable from the root |

Unreachable elements are reported but their references are not checked.

## Mutations

`Ucr::apply` takes a sequence of `MutationOp` values and applies all of them in
order, or none. A failure reports the index of the failing operation and the
reason. A successful call returns `Applied`: the IDs that were inserted,
removed, and updated, and the `state` keys that changed.

| Operation | Effect |
| --- | --- |
| `Insert` | Add a new element to a Container at an index. An inserted Container has no children |
| `Remove` | Remove an element and its descendants. Not allowed for the root |
| `Move` | Reparent or reorder an element. Not allowed for the root or into its own subtree |
| `Update` | Replace position, size, or the attributes of the same element type |
| `SetState`, `RemoveState` | Change one key of the document's `state` |
| `SetData`, `RemoveData` | Change one key of an element's `data` |
| `SetAction`, `RemoveAction` | Change an element's action for one timing |

Operations address only elements reachable from the root. Removing an absent
key or action does nothing.

## State, data, and actions

A document is the source of truth for application state as well as appearance.
`state` holds values that belong to no element; an element's `data` holds values
tied to it and is removed with it. Both map strings to `Value`, which is Bool,
Number, String, List, or Map. There is no null and no schema.

An element's `actions` map a `Timing`, such as `Activated`, to an `FfrCall`
naming a function. `Event` describes something that happened to an element.
This crate only stores and describes them; `seruq-ffr` calls the functions.

See the [UCR v0 draft](../../docs/ucr/v0/README.md) for the specification this
model follows. Colors, Geometry shapes, and image fitting are not modeled yet.
