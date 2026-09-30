use std::error::Error;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DefaultRuntimeError<RendererError, SurfaceError>
where
    RendererError: Error + 'static,
    SurfaceError: Error + 'static,
{
    #[error("failed to create surface: {0}")]
    CreateSurface(#[source] RendererError),
    #[error("failed to render frame: {0}")]
    Render(#[source] RendererError),
    #[error("failed to resize surface: {0}")]
    Resize(#[source] SurfaceError),
    #[error("failed to acquire frame: {0}")]
    Acquire(#[source] SurfaceError),
    #[error("failed to present frame: {0}")]
    Present(#[source] SurfaceError),
}
