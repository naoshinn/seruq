# Seruq UCR Specification and Versioning

UCR is Seruq's language-neutral data model for describing user interfaces and their actions.
The specification defines how UCR data is interpreted; implementations do not need to use the official Rust API.

UCR specifications are organized into a separate directory for each major version.
This document lists the specifications and defines the compatibility and migration policies that apply to all versions.

| Major version | Description | Status |
| --- | --- | --- |
| [v0](v0/README.md) | Pre-release design and validation phase | Design in progress |
| v1 | First official specification | Not yet published |

Start with the [v0 draft](v0/README.md) for the data model, element types, rendering rules, and action handling.
v0 is incomplete: its proposed fields and unspecified behavior are not a released interoperability contract.

## Compatibility

v0 is the design phase before the first official release and does not guarantee backward compatibility.
Breaking changes are allowed while the specification is being developed and validated.

v1 is the first official release. From v1 onward, revisions within the same major version must continue
to accept UCR data valid under earlier revisions and preserve its specified meaning.
This does not require older implementations to support features added by newer revisions.

From v1 onward, changes that remove existing features, invalidate previously valid data, or alter existing
semantics or defaults require a new major version. Adding features must preserve existing data and its meaning.
Published revisions must remain identifiable, for example through Git tags.

## Major Releases and the Official Migrator

Each new major version must meet the following requirements before its official release:

1. Document breaking changes and the migration rules from the previous specification.
2. Provide Seruq UCR Migrator as the official implementation of those rules.
3. Clearly identify changes that cannot be migrated automatically and any required manual steps.
4. Migrate existing UCR data and verify that it conforms to the new specification.

This policy also applies to the transition from v0 to v1; the supported source revisions must be identified.
The official Migrator primarily targets UCR Binary. Migration rules are public and independent of the
official implementation, so other implementations can provide their own Migrators.
