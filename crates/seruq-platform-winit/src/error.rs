use std::{error::Error, fmt};

use winit::error::{EventLoopError, OsError};

#[derive(Debug)]
pub enum WinitPlatformError {
    EventLoop(EventLoopError),
    CreateWindow(OsError),
}

impl fmt::Display for WinitPlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(error) => write!(formatter, "winit event loop failed: {error}"),
            Self::CreateWindow(error) => write!(formatter, "winit window creation failed: {error}"),
        }
    }
}

impl Error for WinitPlatformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EventLoop(error) => Some(error),
            Self::CreateWindow(error) => Some(error),
        }
    }
}
