//! JSON encoding of the current UCR model.
//!
//! Decoding produces a [`UcrDraft`]. Call [`UcrDraft::build`] to validate its
//! reachable UI structure and obtain a validated UCR and warnings.

mod model;
mod scalar;

use std::fmt;

use seruq_ucr::{ElementId, GeometryError, UcrDraft};

/// A JSON syntax, representation, or geometry-construction error.
#[derive(Debug)]
pub enum Error {
    Json(serde_json::Error),
    InvalidGeometry {
        element: ElementId,
        field: &'static str,
        reason: GeometryError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "{error}"),
            Self::InvalidGeometry {
                element,
                field,
                reason,
            } => write!(
                formatter,
                "invalid {field} for element {element}: {reason:?}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::InvalidGeometry { .. } => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Decode JSON without validating references, cycles, or font sizes.
///
/// Point and Size construction constraints are enforced even for unused elements.
pub fn from_str(json: &str) -> Result<UcrDraft, Error> {
    let document: model::UcrJson = serde_json::from_str(json)?;
    document.try_into()
}

/// Encode a draft without requiring its reachable structure to be valid.
pub fn to_string(draft: &UcrDraft) -> Result<String, Error> {
    Ok(serde_json::to_string(&model::UcrJson::from(draft))?)
}

/// Encode a draft as indented JSON.
pub fn to_string_pretty(draft: &UcrDraft) -> Result<String, Error> {
    Ok(serde_json::to_string_pretty(&model::UcrJson::from(draft))?)
}
