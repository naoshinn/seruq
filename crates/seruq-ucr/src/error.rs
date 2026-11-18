//! `Display` and `Error` for the errors of the value types and of mutations,
//! so that functions can propagate them with `?`.

use std::{error::Error, fmt};

use crate::{AssetPathError, FontSizeError, GeometryError, MutationError, NumberError};

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "coordinates and sizes must be finite",
            Self::Negative => "sizes must not be negative",
        })
    }
}

impl fmt::Display for FontSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "font size must be finite",
            Self::NonPositive => "font size must be positive",
        })
    }
}

impl fmt::Display for AssetPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "asset path must not be empty",
            Self::Absolute => "asset path must be relative",
            Self::InvalidSegment => "asset path segments must not be empty, `.`, or `..`",
            Self::InvalidCharacter => {
                "asset path must not contain `\\`, `:`, or control characters"
            }
        })
    }
}

impl fmt::Display for NumberError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "number must be finite",
        })
    }
}

impl fmt::Display for MutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "mutation operation {} failed: {:?}",
            self.index, self.reason
        )
    }
}

impl Error for GeometryError {}
impl Error for FontSizeError {}
impl Error for AssetPathError {}
impl Error for NumberError {}
impl Error for MutationError {}
