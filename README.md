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
The current Rust implementation is an early prototype and does not yet implement this draft.

## License

This repository is licensed under either the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.
