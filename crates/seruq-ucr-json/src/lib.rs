//! JSON encoding of the current UCR model.
//!
//! Decoding produces a [`UcrDraft`]. Call [`UcrDraft::build`] to validate its
//! reachable UI structure and obtain a validated UCR and warnings.
//! Values that the UCR data types cannot represent are rejected while decoding.

mod model;
mod scalar;
mod value;

use std::fmt;

use seruq_ucr::{AssetPathError, ElementId, FontSizeError, GeometryError, UcrDraft};

/// A JSON syntax or representation error, or a value rejected by a UCR data type.
#[derive(Debug)]
pub enum Error {
    Json(serde_json::Error),
    InvalidGeometry {
        element: ElementId,
        field: &'static str,
        reason: GeometryError,
    },
    InvalidFontSize {
        element: ElementId,
        reason: FontSizeError,
    },
    /// `element` is `None` for `metadata.default_font`.
    InvalidAssetPath {
        element: Option<ElementId>,
        field: &'static str,
        reason: AssetPathError,
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
            Self::InvalidFontSize { element, reason } => {
                write!(
                    formatter,
                    "invalid font_size for element {element}: {reason:?}"
                )
            }
            Self::InvalidAssetPath {
                element: Some(element),
                field,
                reason,
            } => write!(
                formatter,
                "invalid {field} path for element {element}: {reason:?}"
            ),
            Self::InvalidAssetPath {
                element: None,
                field,
                reason,
            } => write!(formatter, "invalid metadata {field} path: {reason:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Decode JSON without validating references or cycles.
///
/// Constraints of Point, Size, FontSize, and AssetPath are enforced even for unused elements.
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
