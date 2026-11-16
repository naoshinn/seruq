//! The UCR data model, its validation, and Mutations.
//!
//! Build a [`UcrDraft`] freely, then call [`UcrDraft::build`] to obtain a
//! validated [`Ucr`]. Change a `Ucr` with [`Ucr::apply`], which applies a
//! [`Mutation`] completely or not at all and keeps the document valid.
//!
//! Value constraints are enforced by types such as [`Point`], [`FontSize`],
//! [`AssetPath`], and [`Number`], which reject invalid values on construction.
//! Structural constraints, such as missing references and cycles, are checked by validation.

mod asset;
mod error;
mod event;
mod font_size;
mod geometry;
mod mutation;
mod ucr;
mod validate;
mod value;

pub use asset::{AssetPath, AssetPathError};
pub use event::{Actions, Event, EventKind, FfrCall, Timing};
pub use font_size::{FontSize, FontSizeError};
pub use geometry::{GeometryError, Point, Size};
pub use mutation::{Applied, Mutation, MutationError, MutationErrorReason, MutationOp};
pub use ucr::{Element, ElementId, ElementKind, Metadata, UcrDraft};
pub use validate::{Ucr, ValidationError, ValidationReport, ValidationWarning};
pub use value::{Number, NumberError, Value, ValueMap};
