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
| Application / DSL | Compute layout, run external logic, and decide whether to apply Mutations |

UCR is a low-level representation. It contains what a runtime must interpret and what functions,
possibly written in different languages, must share. Conveniences for authors, such as layout,
typed records, and bindings between data and presentation, belong to the DSLs and libraries that produce UCR.

The first specification uses absolute positioning and has no layout engine.
An element's region is always the `position` and `size` stored in UCR; content never changes it.
Applications handle layout changes when the viewport size changes, using the
[text measurement rules](#41-text-and-fonts), the [control metrics](#45-button-and-textinput),
and the [ViewportResized event](#5-events-and-ffr).

## 2. Abstract Data Model

UCR contains metadata, a reference to a single root, a Map from IDs to Elements, and application state.
The Map's iteration order has no visual meaning.

In the notation below, `Map<K, V>` is a collection keyed by unique values of type `K`,
and `Sequence<T>` is an ordered sequence of values of type `T`.
These describe the abstract model, not particular programming-language types or a storage format.

```text
UCR = {
    metadata: Metadata,
    root: ElementId,
    elements: Map<ElementId, Element>,
    state: Map<String, Value>
}

Element = Container | Text | Image | Button | TextInput | Geometry

Common Element fields = {
    position: { x: LogicalNumber, y: LogicalNumber },
    size: { width: LogicalNumber, height: LogicalNumber },
    actions: Map<Timing, FfrCall>,
    data: Map<String, Value>
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

### 2.2. State and Data

UCR is the source of truth for an application's state, not only for its appearance.
Functions called through FFR read the UCR and return Mutations; they share data with each other through the UCR.
Two places hold application data, and neither affects rendering, placement, or hit testing:

| Place | Holds | Lifetime |
| --- | --- | --- |
| `state` of the UCR | Values that belong to no particular element, such as a counter or the current mode | Until removed explicitly |
| `data` of an Element | Values tied to that element, such as the record a row presents | Removed together with the element |

Both are optional and their absence means an empty map. An element's `data` also serves as the fixed
arguments of its actions: a function finds the event's target and reads its `data`.

```text
Value = Bool | Number | String | Sequence<Value> | Map<String, Value>
Number = finite IEEE 754 binary64
```

- There is no null. A value that is not present is a key that is not present.
- There is one numeric type. Integers are exact up to 2^53; larger identifiers, including ElementIds, are stored as strings.
- Keys are arbitrary strings and are unique within a map. The order of keys has no meaning.
- NaN and infinities produce `InvalidValue`. Like other data type constraints, this applies to the entire document.

UCR has no record types and no schema for `state` or `data`. Languages map their own types, such as structs or classes,
to and from Map values. A value that does not have the shape a function expects is that function's failure,
not a validation error. UCR also does not connect state to presentation: a function that changes a displayed value
returns a Mutation that changes both the state and the elements that show it.

Details still to be specified: Limits on nesting depth and size, and whether a schema can be attached later as an additional constraint.

## 3. Coordinates, Sizes, and Drawing Order

Coordinates and sizes are logical values represented by 32-bit floating-point numbers.
The origin is at the top left; x increases to the right and y increases downward.
A child's position is relative to its parent Container's origin.

The proposed numeric constraints limit numbers to finite IEEE 754 binary32 values. Width and height are nonnegative.
Negative x and y coordinates are allowed. NaN, infinity, and negative sizes produce `InvalidValue`.
A region with zero width or height is empty.

Logical positions are computed by adding the positions of ancestors.
For example, a Container at `(100, 50)` with a child at `(10, 20)` places the child at `(110, 70)`.

The proposed computation rules make placement identical across implementations:

- Accumulated positions are IEEE 754 binary64 values. Each stored binary32 value is converted exactly,
  and positions are added in order from the root toward the element.
- An element's region is the half-open rectangle `[x, x + width)` by `[y, y + height)`.
- These rules apply to logical coordinates. Rasterization rounding after applying the scale factor is not prescribed.

The runtime applies the scale factor supplied by its execution environment, such as the OS, when rendering.
Coordinates stored in UCR do not need to be rewritten as physical pixels.
Tests and independent runtimes can also supply a scale factor.

Child rendering outside a Container's bounds is clipped.
Nested Containers are also subject to clipping by their ancestors.
Containers have transparent backgrounds by default; the overall viewport background is white by default.

Children are drawn in sequence order, with later elements appearing in front of earlier ones.
A child Container's subtree is drawn together at that Container's position in the sequence.
The first specification has no separate `z-index` attribute.

### 3.1. Placed List and Hit Testing

The placed list is the proposed common result of interpreting a UCR's structure.
It contains every element reachable from the root in drawing order, with these values in logical coordinates:

```text
Placed = {
    id: ElementId,
    bounds: Rectangle,            // the element's own region
    visible: Rectangle | None     // bounds intersected with every ancestor Container's bounds
}
```

`visible` is None when the intersection is empty, including when the element has zero width or height.
The viewport is not part of this computation, so the list does not depend on the window size.
Rendering draws the list from first to last. Hit testing searches it from last to first:

- The target of a logical point is the last listed element that is not a Container and whose `visible` region contains the point.
- Containers are transparent to hit testing. A point inside a Container but outside all of its descendants reaches the elements behind it.
- Every other element type is a target and hides the elements behind it, whether or not it handles the event.
- A point with no target hits nothing.

Details still to be specified: Clipping to the bounds of non-Container elements,
whether elements without actions should also become transparent to hit testing,
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
- `font_size` is a positive, finite f32 logical size. Zero, negative, and non-finite values produce `InvalidValue`.
  Text is not automatically scaled to fit its region.
- The abstract string model is a sequence of Unicode scalar values. Invalid UTF-8 is rejected.
- Reading strings does not implicitly perform Unicode normalization.

Applications compute layout, so they must be able to measure text without a particular runtime.
The proposed measurement rules depend only on the font file and the string:

- The width of a line is the sum of the advance widths of its shaped glyphs in font units,
  multiplied by `font_size / units_per_em`. Hinting and pixel snapping do not affect it.
- The height of a line is `font_size × 1.2`. The height of a text is its line count multiplied by that value.
- Lines are separated only by explicit line breaks. There is no automatic wrapping in the first specification.
- Measurement is defined only when the font is resolved from an explicit font file.
  When neither font is provided, an implementation may use a platform font, and measurements are implementation-defined.

A text that is larger than its element's region is clipped, never resized.
The official implementation should provide these rules as a library independent of any runtime,
so that DSLs and runtimes share one implementation. DOM-based implementations may be unable to match
measured widths exactly; the permitted tolerance is unresolved.

Details still to be specified: The shaping features that affect advances, line-break characters, alignment,
missing glyphs, the placement of the baseline within a line, and the measurement tolerance.
This draft does not guarantee identical pixel output for the same `font_size`; see [Conformance](#8-conformance).

### 4.2. Font and Image Asset References

Fonts and images are specified by paths.

The proposed asset model treats a path as a relative reference resolved against an asset base supplied by the application.
The base is not implicitly derived from the UCR file's location or the process's working directory.
Independent implementations can map these references to URLs or packaged assets, for example.

The proposed path syntax keeps every reference inside the asset base:

- A path is a nonempty sequence of segments separated by `/`.
- Segments are nonempty and are not `.` or `..`. A path therefore cannot start or end with `/`.
- `\`, `:`, and control characters are not allowed anywhere in a path.

A path that violates these rules produces `InvalidValue`. Like other data type constraints, this applies to the entire document.

Details still to be specified: The contract for supplying the asset base, supported image and font formats,
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

The proposed control metrics are fixed constants in logical units, so that applications can compute the space available for text:

| Metric | Value |
| --- | --- |
| Border width | 1, drawn inside the element's region |
| Content inset | 4 from each edge of the element's region, including the border |

The label, value, and placeholder are laid out in the content region, which is the element's region reduced by the content inset.
Making these values attributes instead of constants remains open.

Details still to be specified: Disabled states, placeholder visibility, and input state such as IME composition and selection.
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

These timings belong to elements. The proposal adds one document-level event that has no target element:

| Event | Payload | Meaning |
| --- | --- | --- |
| ViewportResized | Logical width and height, scale factor | The viewport size or scale factor has changed |

The runtime delivers ViewportResized to the application, which can respond with a Mutation that repositions elements.
It is also delivered once before the first rendering. Where a document stores the FFR call for this event is unresolved.

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
FFR connects it to the corresponding implementation. An element has at most one FfrCall per Timing.
The ValueChanged payload contains the new string value; the other timings have no payload.

The proposed function contract:

- A function receives the Event and a read-only view of the current UCR, and returns either a Mutation or a failure.
  It reads `state`, the target's `data`, and any other part of the UCR to decide what to return.
- A function does not modify the UCR. Resources that cannot be stored in UCR, such as connections, stay with the function.
- The runtime produces Events and does not know about FFR. Resolving an Event to a function is a separate step,
  so Events remain observable for elements without actions.

The proposed resolution of an Event:

1. If the target no longer exists or is not reachable from the root, the Event is discarded.
2. If the target has no FfrCall for the Event's Timing, nothing is called.
3. Otherwise the function named by the FfrCall is called, and the Mutation it returns is applied before the next Event is processed.

Three failures are distinguished. None of them changes the UCR, and processing continues with the next Event:

| Failure | Meaning |
| --- | --- |
| Unknown function | No function is connected to `function_name` |
| Function failure | The function ran and reported a failure |
| Rejected Mutation | The returned Mutation violates an [application rule](#51-ucr-mutation) |

Each failure identifies the element, the Timing, and the `function_name`.
An implementation should be able to list, when a UCR is loaded, every FfrCall of a reachable element
whose `function_name` has no connected function, so that missing functions are found before any Event occurs.

| Element | Initial supported Timing proposal |
| --- | --- |
| Button | Pushed, Released, Activated, Focused, Unfocused |
| TextInput | Focused, Unfocused, ValueChanged |
| Container, Text, Image, Geometry | No standard events in the first specification |

The first specification proposes synchronous FFR calls: a function receives an Event and returns a Mutation.

Details still to be specified: The final supported event set, event ordering, cancellation notifications,
asynchronous execution, the syntax of `function_name`, how failures are delivered to the application,
and whether an action on an element type that never produces its Timing is a validation warning.

Native data types can be used for passing Mutations, so normal execution does not require encoding and decoding every update.

### 5.1. UCR Mutation

The proposed Mutation is an ordered sequence of operations from a closed set:

```text
Mutation = Sequence<Operation>

Operation = Insert { id: ElementId, element: Element, parent: ElementId, index: Integer }
          | Remove { id: ElementId }
          | Move   { id: ElementId, parent: ElementId, index: Integer }
          | Update { id: ElementId, position?, size?, attributes? }
          | SetState    { key: String, value: Value }
          | RemoveState { key: String }
          | SetData     { id: ElementId, key: String, value: Value }
          | RemoveData  { id: ElementId, key: String }
          | SetAction    { id: ElementId, timing: Timing, call: FfrCall }
          | RemoveAction { id: ElementId, timing: Timing }
```

| Operation | Effect |
| --- | --- |
| Insert | Add a new element to the `children` of `parent` at `index` |
| Remove | Remove an element and all of its descendants from the Map |
| Move | Detach an element and insert it into the `children` of `parent` at `index`, counted after detaching |
| Update | Replace the given position, size, or type-specific attributes of an element; `data` and `actions` are unchanged |
| SetState | Set one key of the UCR's `state`, replacing any existing value |
| RemoveState | Remove one key of the UCR's `state` |
| SetData | Set one key of an element's `data`, replacing any existing value |
| RemoveData | Remove one key of an element's `data` |
| SetAction | Set an element's FfrCall for one Timing, replacing any existing one |
| RemoveAction | Remove an element's FfrCall for one Timing |

Application rules:

- A Mutation is atomic. Operations are applied in order, each to the result of the preceding ones.
  If any operation fails, the UCR is left unchanged and the failure identifies the operation and the reason.
- Applying a Mutation to a valid UCR yields a valid UCR. Each operation fails rather than break a validity rule:
  - Insert fails if the ID already exists, if `parent` is not a reachable Container, or if `index` exceeds the number of children.
    An inserted Container has no children; subtrees are built with further Insert operations.
  - Remove and Move fail when they target the root.
  - Move fails if `parent` is the element itself or one of its descendants.
  - Update cannot change an element's type, and cannot replace a Container's `children`.
  - Every operation fails if it names an element that does not exist or is not reachable from the root.
  - RemoveState, RemoveData, and RemoveAction have no effect when the key or Timing is absent; this is not a failure.
- SetState and SetData replace the whole value of a key. Changing part of a Sequence or Map means setting the key again.
- A function that reads the UCR and returns a Mutation relies on the UCR not changing in between.
  Mutations are therefore applied before the next event is processed, and work that completes later
  re-enters as a new function call that reads the current UCR, rather than as a Mutation computed from an old one.
- A Mutation never creates unreachable elements. An ID freed by Remove can be reused by a later Insert.
- New IDs are chosen by the producer of the Mutation. A collision with any existing ID, including an unreachable element's, is a failure.

The application decides whether and when to apply a Mutation; the rules above define what applying means.
After a Mutation is applied, the runtime discards temporary state that refers to removed elements:
a press in progress on a removed element is canceled without producing Activated, and a removed focused element loses focus.

Details still to be specified: Per-attribute updates, replacing the root or metadata, an encoding of Mutations for UCR JSON and UCR Binary,
and the events produced when temporary state is discarded.

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

Every diagnostic identifies the elements it concerns: the referencing Container and the referenced ID for
reference errors, and the element itself for an unreachable-element warning, with one warning per element.
A reference to the root from the reachable structure is always a cycle; it is reported once, as the root having a parent.

The proposed additional structural constraints prohibit repeating a child ID within the same Container.
Duplicate IDs found while reading data produce an error rather than being silently overwritten when constructing the Map.
Cycle detection covers only the structure reachable from the root.

Validation traverses the structure from the root, checking attributes, references, parent relationships, and cycles.
Elements not reached by this traversal produce a warning.
When a change makes previously unreachable elements reachable, their validity must be checked before accepting the change.

Details still to be specified: Structural error names, the common diagnostic data type, the order of diagnostics,
and handling of unknown Elements and attributes.
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

## 8. Conformance

UCR does not require identical pixels. Requiring them would bind every implementation to one rasterizer,
one text shaper, and one set of rounding rules, which a DOM-based implementation cannot satisfy.
The proposal defines conformance in three levels instead:

| Level | Subject | Requirement |
| --- | --- | --- |
| L1 Data | Decoding and validation | Identical acceptance, and identical diagnostics with the elements they concern |
| L2 Placement and interaction | The [placed list](#31-placed-list-and-hit-testing), hit-test results, and the events produced by a sequence of inputs | Identical |
| L3 Rendering | Pixels | Invariants only |

The proposed L3 invariants: nothing is painted outside an element's `visible` region, elements are painted in placed-list order,
and the interior of a solid fill has exactly the specified color.

Conformance is checked with language-neutral test cases: a UCR JSON document together with its expected diagnostics,
placed list, and, for a sequence of inputs, the expected events. The file formats of these cases are unresolved.

An implementation may guarantee more than this specification requires. The official runtime can aim for identical pixels
across platforms for one version of itself when every font is an explicit font file, and use that for its own regression tests.
That is a property of the implementation, not a conformance requirement.

## 9. Specification Gaps

The following areas need defined behavior before independent implementations can interpret the model consistently:

1. The contract for supplying the asset base, and behavior when no font is specified.
2. Shaping and tolerance in text measurement, and Image fitting and placement.
3. TextInput editing state and its relationship to Mutation application.
4. Geometry path, fill, and stroke rules.
5. Events supported by each Element, their ordering, and how FFR failures are delivered.
6. The file formats of conformance test cases, and an encoding of Mutations.
7. Limits on the size and nesting depth of `state` and `data`.

The document also leaves Binary ID encoding, Color representation, diagnostic types, and the Binary layout unspecified.
These gaps and the proposed designs need to be resolved before v1 is published as an official specification.
