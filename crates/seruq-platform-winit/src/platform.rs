use std::sync::Arc;

use seruq_core::{
    PhysicalSize, Platform, PlatformCommand, PlatformEvent, PlatformHandler, PlatformRunError,
    SurfaceTarget,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::WinitPlatformError;

#[derive(Default)]
pub struct WinitPlatform;

pub struct WinitSurfaceTarget {
    window: Arc<Window>,
}

impl WinitSurfaceTarget {
    pub fn into_window(self) -> Arc<Window> {
        self.window
    }
}

impl SurfaceTarget for WinitSurfaceTarget {}

struct App<'a, H: PlatformHandler<WinitSurfaceTarget>> {
    handler: &'a mut H,
    window: Option<Arc<Window>>,
    error: Option<PlatformRunError<WinitPlatformError, H::Error>>,
    shut_down: bool,
}

impl<H: PlatformHandler<WinitSurfaceTarget>> App<'_, H> {
    fn send(&mut self, event_loop: &ActiveEventLoop, event: PlatformEvent<WinitSurfaceTarget>) {
        if self.error.is_some() {
            return;
        }

        match self.handler.handle_event(event) {
            Ok(commands) => {
                for command in commands {
                    self.execute(event_loop, command);
                    if self.error.is_some() {
                        break;
                    }
                }
            }
            Err(error) => self.fail(event_loop, PlatformRunError::Handler(error)),
        }
    }

    fn execute(&mut self, event_loop: &ActiveEventLoop, command: PlatformCommand) {
        match command {
            PlatformCommand::CreateWindow if self.window.is_none() => {
                match event_loop.create_window(Window::default_attributes()) {
                    Ok(window) => {
                        let window = Arc::new(window);
                        let size = window.inner_size();
                        self.window = Some(Arc::clone(&window));
                        self.send(
                            event_loop,
                            PlatformEvent::WindowCreated {
                                target: WinitSurfaceTarget { window },
                                size: PhysicalSize::new(size.width, size.height),
                            },
                        );
                    }
                    Err(error) => self.fail(
                        event_loop,
                        PlatformRunError::Platform(WinitPlatformError::CreateWindow(error)),
                    ),
                }
            }
            PlatformCommand::CreateWindow => {}
            PlatformCommand::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            PlatformCommand::Exit => event_loop.exit(),
        }
    }

    fn fail(
        &mut self,
        event_loop: &ActiveEventLoop,
        error: PlatformRunError<WinitPlatformError, H::Error>,
    ) {
        self.error = Some(error);
        event_loop.exit();
    }

    fn shutdown(&mut self) {
        if !self.shut_down {
            self.handler.shutdown();
            self.shut_down = true;
        }
    }
}

impl<H: PlatformHandler<WinitSurfaceTarget>> ApplicationHandler for App<'_, H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.send(event_loop, PlatformEvent::Resumed);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.send(event_loop, PlatformEvent::Suspended);
        self.window = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => self.send(event_loop, PlatformEvent::RedrawRequested),
            WindowEvent::Resized(size) => self.send(
                event_loop,
                PlatformEvent::Resized {
                    size: PhysicalSize::new(size.width, size.height),
                },
            ),
            WindowEvent::CloseRequested => self.send(event_loop, PlatformEvent::CloseRequested),
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown();
    }
}

impl Platform for WinitPlatform {
    type Target = WinitSurfaceTarget;
    type Error = WinitPlatformError;

    fn run<H>(self, handler: &mut H) -> Result<(), PlatformRunError<Self::Error, H::Error>>
    where
        H: PlatformHandler<Self::Target>,
    {
        let event_loop = match EventLoop::new() {
            Ok(event_loop) => event_loop,
            Err(error) => {
                handler.shutdown();
                return Err(PlatformRunError::Platform(WinitPlatformError::EventLoop(
                    error,
                )));
            }
        };

        let mut app = App {
            handler,
            window: None,
            error: None,
            shut_down: false,
        };
        let result = event_loop.run_app(&mut app);
        app.shutdown();

        if let Some(error) = app.error {
            return Err(error);
        }
        result.map_err(|error| PlatformRunError::Platform(WinitPlatformError::EventLoop(error)))
    }
}
