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
pub enum PlatformRunError<PlatformError, HandlerError> {
    Platform(PlatformError),
    Handler(HandlerError),
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
