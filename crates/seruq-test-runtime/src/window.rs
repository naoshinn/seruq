use std::{num::NonZeroU32, rc::Rc};

use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::{Error, Runtime};

/// Window dimensions are logical units, independent of the document's root size.
#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "Seruq Test Runtime".into(),
            width: 640,
            height: 480,
        }
    }
}

pub(crate) fn run(runtime: Runtime, options: WindowOptions) -> Result<(), Error> {
    if options.width == 0 || options.height == 0 {
        return Err(Error::InvalidViewport {
            width: options.width,
            height: options.height,
        });
    }
    let event_loop = EventLoop::new().map_err(Error::window)?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        runtime,
        options,
        surface: None,
        error: None,
    };
    event_loop.run_app(&mut app).map_err(Error::window)?;

    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct App {
    runtime: Runtime,
    options: WindowOptions,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    error: Option<Error>,
}

impl App {
    fn create_surface(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Error> {
        let attributes = Window::default_attributes()
            .with_title(&self.options.title)
            .with_inner_size(LogicalSize::new(self.options.width, self.options.height));
        let window = Rc::new(
            event_loop
                .create_window(attributes)
                .map_err(Error::window)?,
        );
        let context = Context::new(window.clone()).map_err(Error::window)?;
        let surface = Surface::new(&context, window.clone()).map_err(Error::window)?;

        window.request_redraw();
        self.surface = Some(surface);
        Ok(())
    }

    fn redraw(&mut self) -> Result<(), Error> {
        let Some(surface) = self.surface.as_mut() else {
            return Ok(());
        };
        let window = surface.window().clone();
        let size = window.inner_size();
        // Minimized windows can have a zero-sized drawable area.
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };

        let frame = self
            .runtime
            .render(width.get(), height.get(), window.scale_factor())?;
        surface.resize(width, height).map_err(Error::window)?;
        let mut buffer = surface.buffer_mut().map_err(Error::window)?;
        // Convert the renderer's RGBA bytes to softbuffer's 0x00RRGGBB pixels.
        for (pixel, rgba) in buffer.iter_mut().zip(frame.rgba().as_chunks::<4>().0) {
            *pixel = (u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2]);
        }
        window.pre_present_notify();
        buffer.present().map_err(Error::window)?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface.is_none()
            && let Err(error) = self.create_surface(event_loop)
        {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        if surface.window().id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                surface.window().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface = None;
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface = None;
    }
}
