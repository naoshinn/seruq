use std::{error::Error, fmt};

use crate::{PhysicalSize, surface::SurfaceTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformCommand {
    CreateWindow,
    RequestRedraw,
    Exit,
}

#[derive(Debug, PartialEq)]
pub enum PlatformEvent<T: SurfaceTarget> {
    Resumed,
    WindowCreated { target: T, size: PhysicalSize },
    RedrawRequested,
    Resized { size: PhysicalSize },
    CloseRequested,
    Suspended,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum PlatformRunError<PlatformError, HandlerError> {
    Platform(PlatformError),
    Handler(HandlerError),
}

impl<PlatformError, HandlerError> fmt::Display for PlatformRunError<PlatformError, HandlerError>
where
    PlatformError: fmt::Display,
    HandlerError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(error) => write!(formatter, "platform failed: {error}"),
            Self::Handler(error) => write!(formatter, "handler failed: {error}"),
        }
    }
}

impl<PlatformError, HandlerError> Error for PlatformRunError<PlatformError, HandlerError>
where
    PlatformError: Error + 'static,
    HandlerError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Platform(error) => Some(error),
            Self::Handler(error) => Some(error),
        }
    }
}

pub trait Platform {
    type Target: SurfaceTarget;
    type Error;

    fn run<H>(self, handler: &mut H) -> Result<(), PlatformRunError<Self::Error, H::Error>>
    where
        Self: Sized,
        H: PlatformHandler<Self::Target>;
}

pub trait PlatformHandler<T: SurfaceTarget> {
    type Error;

    fn handle_event(
        &mut self,
        event: PlatformEvent<T>,
    ) -> Result<Vec<PlatformCommand>, Self::Error>;

    fn shutdown(&mut self);
}
