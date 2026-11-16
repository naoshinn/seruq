//! A provisional CPU renderer and window host for validated Seruq UCR documents.
//!
//! Applications own decoding and validation. Pass a validated [`Ucr`] and an
//! explicit asset base to [`Runtime::new`], then render offscreen or open a window.
//! The window host requires the default `window` feature.

mod error;
mod layout;
mod render;
mod text;
#[cfg(feature = "window")]
mod window;

use std::path::{Path, PathBuf};

use seruq_ucr::{Applied, ElementId, MutationError, MutationOp, Ucr};
use tiny_skia::Pixmap;

pub use error::Error;
pub use layout::{Bounds, Placed};
#[cfg(feature = "window")]
pub use window::WindowOptions;

/// Owns a validated document and caches its image and font resources.
pub struct Runtime {
    document: Ucr,
    renderer: render::Renderer,
}

impl Runtime {
    /// Relative image and font paths are resolved against `asset_base`.
    pub fn new(document: Ucr, asset_base: impl Into<PathBuf>) -> Self {
        Self {
            document,
            renderer: render::Renderer::new(asset_base.into()),
        }
    }

    pub fn document(&self) -> &Ucr {
        &self.document
    }

    /// Apply a mutation to the document, or leave it unchanged on failure.
    pub fn apply(&mut self, mutation: &[MutationOp]) -> Result<Applied, MutationError> {
        self.document.apply(mutation)
    }

    /// Every reachable element in paint order, in logical coordinates.
    ///
    /// Rendering and hit testing both derive from this list.
    pub fn layout(&self) -> Vec<Placed> {
        layout::layout(&self.document)
    }

    /// The frontmost non-Container element at a logical point.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<ElementId> {
        layout::hit_test(&self.document, x, y)
    }

    /// Render a physical-pixel viewport without creating a window.
    ///
    /// `scale_factor` converts the document's logical coordinates to pixels.
    pub fn render(&mut self, width: u32, height: u32, scale_factor: f64) -> Result<Frame, Error> {
        self.renderer
            .render(&self.document, width, height, scale_factor)
    }

    /// Open a desktop window and display the document. Call from the main thread.
    #[cfg(feature = "window")]
    pub fn run(self, options: WindowOptions) -> Result<(), Error> {
        window::run(self, options)
    }
}

/// An opaque white-backed RGBA frame, with rows stored from top to bottom.
pub struct Frame {
    pixmap: Pixmap,
}

impl Frame {
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Four bytes per pixel, in RGBA order. The white background makes alpha 255.
    pub fn rgba(&self) -> &[u8] {
        self.pixmap.data()
    }

    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        self.pixmap
            .save_png(path)
            .map_err(|source| Error::SaveImage {
                path: path.to_owned(),
                source: Box::new(source),
            })
    }
}
