# Seruq UCR v0 Draft

## 1. Purpose and Boundaries

The specification is independent of programming languages, the official API, and any particular rendering system.
An implementation that interprets UCR through the DOM, for example, does not need to depend on `seruq-api`.

Responsibilities are divided as follows:

| Component | Responsibility |
| --- | --- |
| UCR specification | Define data structure, semantics, validity, and compatibility |
| UCR Binary / UCR JSON | Define encodings of the abstract model |
| UCR migration specification | Define conversion rules between versions |
| Official Migrator | Implement the migration rules, primarily for Binary assets |
| Runtime | Handle rendering, clipping, input, and event delivery |
| Application / DSL | Compute layout, run external logic, and apply Mutations |

The first specification uses absolute positioning. Applications handle layout changes when the viewport size changes.

## 2. Abstract Data Model

UCR contains metadata, a reference to a single root, and a Map from IDs to Elements.
The Map's iteration order has no visual meaning.

In the notation below, `Map<K, V>` is a collection keyed by unique values of type `K`,
and `Sequence<T>` is an ordered sequence of values of type `T`.
These describe the abstract model, not particular programming-language types or a storage format.

```text
UCR = {
    metadata: Metadata,
    root: ElementId,
    elements: Map<ElementId, Element>
}

Element = Container | Text | Image | Button | TextInput | Geometry

Common Element fields = {
    position: { x: LogicalNumber, y: LogicalNumber },
    size: { width: LogicalNumber, height: LogicalNumber },
    actions: Map<Timing, FfrCall>
}

Container-specific fields = {
    children: Sequence<ElementId>
}
```

`ElementId` identifies an element as a Map key. The element itself does not need a duplicate ID field.
The proposal makes `actions` optional and interprets its absence as an empty map.

The root can be any Element type; it does not have to be a Container.
Its position is relative to the top-left corner of the viewport.
Its size is explicit and does not automatically follow the window size.

Metadata records the UCR version and can provide `default_font` for elements that omit their own font.
The version representation and additional metadata fields remain unspecified.

For example, a document with a title and a confirmation button can have this structure:

```text
root: panel
elements:
    panel: Container(children: [title, confirm], position: (0, 0), size: (320, 160))
    title: Text(text: "Example", position: (16, 16), size: (288, 32), font_size: 24)
    confirm: Button(label: "Confirm", position: (16, 64), size: (120, 40), font_size: 16,
                    actions: { Activated: { function_name: "app.confirm" } })
```

This is an illustration, not UCR JSON or a complete encoded document.
The IDs are symbolic placeholders; the example uses the proposed attribute definitions below
and assumes metadata supplies a font. Map entry order does not affect drawing order;
the `panel.children` sequence does.

Activating `confirm` asks the application's FFR to resolve and call `app.confirm`.
The function returns a Mutation describing a UI change. The application applies that change to UCR,
and the runtime renders the updated document. The call description itself does not modify the document.

### 2.1. IDs and Parent–Child Relationships

- IDs are unsigned 64-bit integers, unique within a UCR, and remain unchanged when an element moves or its attributes change.
- IDs can be generated with a low collision probability. The generation method is left to implementations.
- Only Containers can have child Elements in the first specification.
- `children` is an ordered sequence of IDs. Every child referenced by a reachable Container must exist in the Map.
- Cycles and multiple parents are prohibited within the structure reachable from the root.
  The root has no parent within that structure; references from unreachable Containers do not count as parents.
- Elements unreachable from the root produce a warning and are excluded from attribute and graph validity checks.

The current experimental UCR JSON ID encoding is described in the [JSON mapping](json.md).
ID encoding in UCR Binary remains unspecified.
Rules for assigning new IDs when duplicating independent elements and updating related references must also be defined.

## 3. Coordinates, Sizes, and Drawing Order

Coordinates and sizes are logical values represented by 32-bit floating-point numbers.
The origin is at the top left; x increases to the right and y increases downward.
A child's position is relative to its parent Container's origin.

The proposed numeric constraints limit numbers to finite IEEE 754 binary32 values. Width and height are nonnegative.
Negative x and y coordinates are allowed. NaN, infinity, and negative sizes produce `InvalidValue`.
A region with zero width or height is empty.

Logical positions are computed by adding the positions of ancestors.
For example, a Container at `(100, 50)` with a child at `(10, 20)` places the child at `(110, 70)`.
This draft does not prescribe internal computation precision or rasterization rounding.

The runtime applies the scale factor supplied by its execution environment, such as the OS, when rendering.
Coordinates stored in UCR do not need to be rewritten as physical pixels.
Tests and independent runtimes can also supply a scale factor.

Child rendering outside a Container's bounds is clipped.
Nested Containers are also subject to clipping by their ancestors.
Containers have transparent backgrounds by default; the overall viewport background is white by default.

Children are drawn in sequence order, with later elements appearing in front of earlier ones.
A child Container's subtree is drawn together at that Container's position in the sequence.
The first specification has no separate `z-index` attribute.

Details still to be specified: Clipping to the bounds of non-Container elements, hit-test boundaries and their relationship to drawing order,
and the field used to specify the overall viewport background.

## 4. Element Attributes and Rendering Rules

This section proposes attribute definitions for the six Element types in the model.
The common `position` and `size` fields are required. Automatic sizing based on content is outside the first specification.

| Element | Required specific attributes | Optional specific attributes | Initial rendering proposal |
| --- | --- | --- | --- |
| Container | `children` | `background` | Draw children in order; transparent background |
| Text | `text`, `font_size` | `font`, `color` | Draw text from the top left of the specified region; black by default |
| Image | `source` | `fit` | Display an image in the specified region |
| Button | `label`, `font_size` | `font`, `color`, `background` | Display a text label as a leaf element |
| TextInput | `value`, `font_size` | `font`, `color`, `background`, `placeholder` | Display and edit a string as a leaf element |
| Geometry | `shape` | `fill`, `stroke` | Draw shapes in the element's local coordinate system |

### 4.1. Text and Fonts

Text content and its font can be specified.
`metadata.default_font` supplies a font when an element omits its own font.
Strings in UCR Binary and UCR JSON use UTF-8 encoding.
Internal string representations are left to each programming language.

The following design is proposed:

- Resolve fonts from the element's `font` first, then `metadata.default_font`.
- Apply the same font resolution to Button and TextInput text.
- `font_size` is a positive, finite f32 logical size. Text is not automatically scaled to fit its region.
- The abstract string model is a sequence of Unicode scalar values. Invalid UTF-8 is rejected.
- Reading strings does not implicitly perform Unicode normalization.

Details still to be specified: Behavior when neither font is provided, wrapping, line breaks, line height, alignment,
missing glyphs, and the precise meaning of font size.
This draft does not guarantee identical pixel output for the same `font_size`.

### 4.2. Font and Image Asset References

Fonts and images are specified by paths.

The proposed asset model treats a path as a relative reference resolved against an asset base supplied by the application.
The base is not implicitly derived from the UCR file's location or the process's working directory.
Independent implementations can map these references to URLs or packaged assets, for example.

Details still to be specified: Path syntax, the contract for supplying the asset base, supported image and font formats,
diagnostics for missing or unreadable assets, and embedded assets.
A missing asset is distinct from `UnknownId`, which concerns Element references.

The first specification's Image fitting modes will be selected from these candidates:

| Candidate | Rendering rule |
| --- | --- |
| Contain | Preserve the aspect ratio and fit the entire image within the region |
| Cover | Preserve the aspect ratio, cover the region, and clip excess content |
| Stretch | Scale the image independently along each axis to fill the region |

The proposed default is Contain with centered placement. Whether all candidates belong in the first specification remains open.

### 4.3. Colors and Backgrounds

The overall viewport background is white by default, and Container backgrounds are transparent by default.
Text and shapes are drawn in black by default.

The proposed defaults give Button and TextInput white backgrounds. Text, Image, and Geometry do not paint their own backgrounds.
Not painting a background is distinct from a black or white Color value.
Images use their own colors.

Details still to be specified: The Color data type, color space, opacity, and compositing rules.
Notation such as `#ffffff` is used for readability and does not define an official encoding.

### 4.4. Geometry

Geometry draws on a shape model similar to SVG.
Drawing commands are structured data, not child Elements.

The following design is proposed:

```text
Shape = Rect | Ellipse | Path
Path = Sequence<MoveTo | LineTo | CubicTo | Close>

Stroke = {
    color: Color,
    width: positive finite LogicalNumber
}
```

The first specification covers solid-color fills and basic strokes. The default fill is black, with no stroke by default.
Coordinates inside Geometry are relative to the element's top-left corner.
The proposal does not implicitly scale the shape when the element's size changes.

Details still to be specified: Fields for each Shape and command, path validity, fill rules, line caps and joins,
path closure semantics, rendering outside the element's bounds, and the scope of SVG conversion.
Full SVG interpretation is not a requirement for UCR implementations.

### 4.5. Button and TextInput

These elements have no child Elements in the first specification.
A Button's content is a string label. Events can trigger FFR calls.

The proposed rendering rules center Button labels within their regions.
TextInput initially supports single-line input, displaying text from the left side of its region.

Details still to be specified: Padding, borders, disabled states, placeholder visibility, and input state such as IME composition and selection.
Rendering between a TextInput edit and the application's application of a Mutation is also undecided.
One candidate keeps the UCR `value` under application control while the runtime maintains temporary editing state.
This requires rules for committing or discarding temporary state and reconciling it with external updates.

## 5. Events and FFR

The Foreign Function Runner (FFR) connects function identifiers in UCR to callable implementations supplied by the application.
UCR stores the call description; it does not embed the function's implementation.
A UCR Mutation is data describing changes to UCR. The application controls whether and when those changes are applied.

The event model includes these candidates:

| Timing | Meaning |
| --- | --- |
| Pushed | A press operation has started |
| Released | The press has been released |
| Activated | The element's operation has been invoked |
| Focused | The element has gained focus |
| Unfocused | The element has lost focus |
| ValueChanged | The input value has changed |

Activated is a single activation event and has no corresponding Unactivated event.
It can also be used when activating a Button through a keyboard or console.
A canceled press does not produce Activated.

`actions` maps Timing values to FFR calls.
Functions return UCR Mutations, which the application decides whether to apply and then applies.
Functions do not normally modify UCR directly.
UCR does not impose implementation-specific permissions or state management that require the official API.

The following design is proposed:

```text
FfrCall = { function_name: String }

Event = {
    target: ElementId,
    timing: Timing,
    payload: data appropriate to Timing
}
```

`function_name` is an identifier independent of actual function names in individual languages.
FFR connects it to the corresponding implementation. The proposal passes a common Event argument to FFR.
The ValueChanged payload contains the new string value.

| Element | Initial supported Timing proposal |
| --- | --- |
| Button | Pushed, Released, Activated, Focused, Unfocused |
| TextInput | Focused, Unfocused, ValueChanged |
| Container, Text, Image, Geometry | No standard events in the first specification |

Details still to be specified: The final supported event set, event ordering, cancellation notifications,
FFR completion, failure and asynchronous execution, fixed arguments, unknown `function_name` values,
and behavior when no action is configured. The event argument structure also needs to be finalized from the proposal above.

UCR Mutation requires a separate specification for its operations and application rules.
This document describes its role in the action flow; those rules have not yet been defined.
Native data types can be used for passing Mutations, so normal execution does not require encoding and decoding every update.

## 6. Validity and the Validator

An official validator will detect cycles and other violations.
Validation rules are documented so independent implementations can check the same structural conditions.

Attribute and graph validity checks apply only to elements reachable from the root.
Unreachable elements produce a warning, but their attribute values and outgoing references are not validated.
References from unreachable Containers do not count as parents of reachable elements.
Parsing requirements and constraints enforced by individual data types still apply to the entire document.

| Condition | Result |
| --- | --- |
| `root` or `children` references a nonexistent ID | UnknownId |
| A non-Container has children | Structural error |
| A self-reference or cycle exists | Structural error |
| The root has a parent, or an element has multiple parents | Structural error |
| An element is unreachable from the root | Warning |
| An attribute violates its type, range, or constraints | InvalidValue |

The proposed additional structural constraints prohibit repeating a child ID within the same Container.
Duplicate IDs found while reading data produce an error rather than being silently overwritten when constructing the Map.
Cycle detection covers only the structure reachable from the root.

Validation traverses the structure from the root, checking attributes, references, parent relationships, and cycles.
Elements not reached by this traversal produce a warning.
When a change makes previously unreachable elements reachable, their validity must be checked before accepting the change.

Details still to be specified: Structural error names, the common diagnostic data type, and handling of unknown Elements and attributes.
Asset resolution and FFR connection failures are reported separately from structural validation of UCR data.

## 7. Versions, Storage Formats, and Migration

Metadata records the UCR version.
Specification versions, Binary encoding versions, and Rust API versions are distinct.

v0 precedes the first official release, and v1 is the first official release.
From v1 onward, backward compatibility is guaranteed within each major version.
Major releases include migration rules and an official Migrator.
This policy also applies to the transition from v0 to v1.

UCR Binary primarily targets compact storage and efficient loading; UCR JSON targets readability and debugging.
Official conversion between the two formats is planned. Both formats must be able to represent the same UCR semantics.
An [experimental JSON mapping](json.md) describes the subset supported by the current Rust implementation.

Migration rules are defined in documents. The official Migrator implements those rules, primarily with Binary input and output.
Whether it uses an intermediate common model or converts Binary directly is an implementation choice.

Details still to be specified: The type of version values, supported version ranges, responses to unknown versions,
finalization of the experimental JSON mapping, the Binary layout, extension identification, and migration rules.

## 8. Specification Gaps

The following areas need defined behavior before independent implementations can interpret the model consistently:

1. The resolution base for font and image paths, and behavior when no font is specified.
2. Text wrapping and line height, and Image fitting and placement.
3. TextInput editing state and its relationship to Mutation application.
4. Geometry path, fill, and stroke rules.
5. Events supported by each Element and the FFR argument and result contracts.

The document also leaves Binary ID encoding, Color representation, diagnostic types, and the Binary layout unspecified.
These gaps and the proposed designs need to be resolved before v1 is published as an official specification.
