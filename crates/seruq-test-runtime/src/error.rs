use std::{fmt, path::PathBuf};

use seruq_ucr::ElementId;

#[derive(Debug)]
pub enum Error {
    InvalidViewport {
        width: u32,
        height: u32,
    },
    InvalidScaleFactor(f64),
    InvalidLayout {
        element: ElementId,
    },
    Asset {
        element: ElementId,
        path: PathBuf,
        source: Box<dyn std::error::Error>,
    },
    InvalidFont {
        element: ElementId,
        path: PathBuf,
    },
    NoFonts {
        element: ElementId,
    },
    UnsupportedElement {
        element: ElementId,
        kind: &'static str,
    },
    SaveImage {
        path: PathBuf,
        source: Box<dyn std::error::Error>,
    },
    Window(Box<dyn std::error::Error>),
}

impl Error {
    #[cfg(feature = "window")]
    pub(crate) fn window(source: impl std::error::Error + 'static) -> Self {
        Self::Window(Box::new(source))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidViewport { width, height } => {
                write!(formatter, "cannot allocate viewport {width}x{height}")
            }
            Self::InvalidScaleFactor(scale) => write!(formatter, "invalid scale factor {scale}"),
            Self::InvalidLayout { element } => write!(
                formatter,
                "element {element} exceeds the renderer's coordinate or text-metric range"
            ),
            Self::Asset {
                element,
                path,
                source,
            } => write!(
                formatter,
                "cannot load asset {} for element {element}: {source}",
                path.display()
            ),
            Self::InvalidFont { element, path } => write!(
                formatter,
                "no usable font face in {} for element {element}",
                path.display()
            ),
            Self::NoFonts { element } => write!(
                formatter,
                "no system fonts available for element {element}; specify a font path"
            ),
            Self::UnsupportedElement { element, kind } => write!(
                formatter,
                "{kind} rendering is not implemented for element {element}"
            ),
            Self::SaveImage { path, source } => {
                write!(formatter, "cannot save {}: {source}", path.display())
            }
            Self::Window(source) => write!(formatter, "window error: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Asset { source, .. } | Self::SaveImage { source, .. } | Self::Window(source) => {
                Some(source.as_ref())
            }
            _ => None,
        }
    }
}
