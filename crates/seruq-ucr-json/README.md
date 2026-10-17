# seruq-ucr-json

An experimental JSON codec for the current Seruq UCR model.

- `from_str(&str)` reads JSON into `UcrDraft`.
- `to_string(&UcrDraft)` writes compact JSON.
- `to_string_pretty(&UcrDraft)` writes indented JSON.

To save a validated `Ucr`, pass `ucr.data()` to either writer.
Decoding enforces Point and Size constraints, but leaves UI structure validation to
`UcrDraft::validate()` or `UcrDraft::build()`. This permits inspecting and saving
drafts with dangling references, cycles, or invalid font sizes.

Serde support and JSON-specific types live in this crate. `seruq-ucr` does not
depend on Serde. The JSON types are implementation details, not a public API.

See the [experimental JSON mapping](../../docs/ucr/v0/json.md) for the data layout,
ID and floating-point representations, and decoding rules. There is no JSON Schema
yet; the mapping can change during v0 development.
