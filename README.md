# Seruq

Seruq is developing UCR, a language-neutral representation of user interfaces.
UI libraries and domain-specific languages can produce UCR for different runtimes to interpret,
including native renderers and DOM-based implementations.

## Status

Seruq is still under development. The API, including schemas, may change frequently and without notice.

## Design

The [UCR specification index](docs/ucr/README.md) describes versioning, backward compatibility, and migration policy.
The [v0 draft](docs/ucr/v0/README.md) introduces the UI model, element attributes, events, and validation rules.
It is incomplete and contains proposed designs and unresolved details.
The current Rust implementation is an early prototype and implements only part of this draft.

## Crates

| Crate | Purpose |
| --- | --- |
| [`seruq-ucr`](crates/seruq-ucr) | The UCR data model, validation, and Mutations. No dependencies |
| [`seruq-ucr-json`](crates/seruq-ucr-json) | An experimental JSON encoding of UCR |
| [`seruq-ffr`](crates/seruq-ffr) | Connects the function names in UCR actions to functions and calls them |
| [`seruq-test-runtime`](crates/seruq-test-runtime) | A provisional CPU renderer, hit tester, and window host |

The other crates depend only on `seruq-ucr`, except in their tests and examples.
A document can be loaded, validated, rendered, and changed by functions called through FFR.
Buttons respond to the pointer, and viewport callbacks let applications update layout on resize.
Viewport and Container backgrounds, control backgrounds, and text colors are configurable.
Keyboard input, focus, and text editing are not implemented yet.

```sh
# Render the sample scene in a window, or to a PNG without one.
cargo run -p seruq-test-runtime --example viewer
cargo run -p seruq-test-runtime --example viewer -- crates/seruq-test-runtime/examples/scene.json preview.png

# Open a counter whose buttons call functions through FFR.
cargo run -p seruq-test-runtime --example interactive_counter

# Run the smallest FFR example, without a window.
cargo run -p seruq-ffr --example counter
```

## License

This repository is licensed under either the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.
