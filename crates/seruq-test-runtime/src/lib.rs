//! A provisional CPU renderer and window host for validated Seruq UCR documents.
//!
//! Applications own decoding and validation. Pass a validated [`Ucr`] and an
//! explicit asset base to [`Runtime::new`], then render offscreen or open a window.
//! The window host requires the default `window` feature.
//!
//! [`Runtime::handle_input`] turns pointer input into UCR events without a
//! window. The runtime does not call functions; the application decides what
//! each event does, typically by resolving it through FFR and passing the
//! resulting mutation to [`Runtime::apply`].

mod error;
mod input;
mod layout;
mod render;
mod text;
#[cfg(feature = "window")]
mod window;

use std::path::{Path, PathBuf};

use seruq_ucr::{Applied, ElementId, Event, MutationError, MutationOp, Ucr};
use tiny_skia::Pixmap;

pub use error::Error;
pub use input::Input;
pub use layout::{Bounds, Placed};
#[cfg(feature = "window")]
pub use window::WindowOptions;

/// Owns a validated document and caches its image and font resources.
pub struct Runtime {
    document: Ucr,
    renderer: render::Renderer,
    interaction: input::Interaction,
}

impl Runtime {
    /// Relative image and font paths are resolved against `asset_base`.
    pub fn new(document: Ucr, asset_base: impl Into<PathBuf>) -> Self {
        Self {
            document,
            renderer: render::Renderer::new(asset_base.into()),
            interaction: input::Interaction::default(),
        }
    }

    pub fn document(&self) -> &Ucr {
        &self.document
    }

    /// Apply a mutation to the document, or leave it unchanged on failure.
    ///
    /// A press in progress on an element that the mutation removes is dropped
    /// without producing further events.
    pub fn apply(&mut self, mutation: &[MutationOp]) -> Result<Applied, MutationError> {
        let applied = self.document.apply(mutation)?;
        self.interaction.reconcile(&self.document, &applied);
        Ok(applied)
    }

    /// Advance the input state and return the events the input produced, in order.
    ///
    /// Pressing a Button produces `Pushed`. Releasing produces `Released`,
    /// followed by `Activated` if the pointer is still over the same Button.
    /// A cancelled press produces only `Released`. Events do not depend on
    /// whether the element has actions.
    pub fn handle_input(&mut self, input: Input) -> Vec<Event> {
        self.interaction.handle(&self.document, input)
    }

    /// The Button currently drawn as pressed: held down with the pointer over it.
    pub fn pressed(&self) -> Option<ElementId> {
        self.interaction.pressed(&self.document)
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
        let pressed = self.pressed();
        self.renderer
            .render(&self.document, width, height, scale_factor, pressed)
    }

    /// Open a desktop window and display the document. Call from the main thread.
    ///
    /// Events are produced and discarded; use [`Runtime::run_with`] to handle them.
    #[cfg(feature = "window")]
    pub fn run(self, options: WindowOptions) -> Result<(), Error> {
        window::run(self, options, |_, _| {})
    }

    /// Open a desktop window and pass every event to `handler`, one at a time.
    ///
    /// The handler can read the document and call [`Runtime::apply`]; the
    /// window is redrawn afterwards. Call from the main thread.
    #[cfg(feature = "window")]
    pub fn run_with(
        self,
        options: WindowOptions,
        handler: impl FnMut(&mut Runtime, Event),
    ) -> Result<(), Error> {
        window::run(self, options, handler)
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
