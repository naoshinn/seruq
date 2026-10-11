use std::{error::Error, mem};

use seruq_core::{
    PhysicalSize, PlatformCommand, PlatformEvent, PlatformHandler, PresentableFrame, Renderer,
    Surface, SurfaceTarget, Ucr, project_ucr,
};

use crate::DefaultRuntimeError;

type EventResult<RendererError, SurfaceError> =
    Result<Vec<PlatformCommand>, DefaultRuntimeError<RendererError, SurfaceError>>;

enum State<T, S> {
    Idle,
    AwaitingWindow,
    AwaitingSurface { target: T },
    Ready { surface: S, size: PhysicalSize },
    Finished,
}

pub struct DefaultRuntime<T, R>
where
    T: SurfaceTarget,
    R: Renderer<T>,
{
    ucr: Ucr,
    renderer: R,
    state: State<T, R::Surface>,
}

impl<T, R> DefaultRuntime<T, R>
where
    T: SurfaceTarget,
    R: Renderer<T>,
    R::Error: Error + 'static,
    <R::Surface as Surface>::Error: Error + 'static,
{
    pub fn new(ucr: Ucr, renderer: R) -> Self {
        Self {
            ucr,
            renderer,
            state: State::Idle,
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.state, State::Finished)
    }

    fn activate_surface(
        &mut self,
        target: T,
        size: PhysicalSize,
    ) -> EventResult<R::Error, <R::Surface as Surface>::Error> {
        if size.is_empty() {
            self.state = State::AwaitingSurface { target };
            return Ok(Vec::new());
        }

        match self.renderer.create_surface(target, size) {
            Ok(surface) => {
                self.state = State::Ready { surface, size };
                Ok(vec![PlatformCommand::RequestRedraw])
            }
            Err(failure) => {
                let (target, error) = failure.into_parts();
                self.state = State::AwaitingSurface { target };
                Err(DefaultRuntimeError::CreateSurface(error))
            }
        }
    }

    fn handle_resumed(&mut self) -> Vec<PlatformCommand> {
        if matches!(self.state, State::Idle) {
            self.state = State::AwaitingWindow;
            vec![PlatformCommand::CreateWindow]
        } else {
            Vec::new()
        }
    }

    fn handle_window_created(
        &mut self,
        target: T,
        size: PhysicalSize,
    ) -> EventResult<R::Error, <R::Surface as Surface>::Error> {
        if !matches!(self.state, State::AwaitingWindow) {
            return Ok(Vec::new());
        }

        self.activate_surface(target, size)
    }

    fn handle_resized(
        &mut self,
        size: PhysicalSize,
    ) -> EventResult<R::Error, <R::Surface as Surface>::Error> {
        match mem::replace(&mut self.state, State::Idle) {
            State::AwaitingSurface { target } => self.activate_surface(target, size),
            State::Ready {
                mut surface,
                size: current,
            } => {
                if size == current || size.is_empty() {
                    self.state = State::Ready { surface, size };
                    return Ok(Vec::new());
                }

                match surface.resize(size) {
                    Ok(()) => {
                        self.state = State::Ready { surface, size };
                        Ok(vec![PlatformCommand::RequestRedraw])
                    }
                    Err(error) => {
                        self.state = State::Ready {
                            surface,
                            size: current,
                        };
                        Err(DefaultRuntimeError::Resize(error))
                    }
                }
            }
            other => {
                self.state = other;
                Ok(Vec::new())
            }
        }
    }

    fn handle_redraw_requested(&mut self) -> EventResult<R::Error, <R::Surface as Surface>::Error> {
        let ucr = &self.ucr;
        let renderer = &mut self.renderer;

        let State::Ready { surface, size } = &mut self.state else {
            return Ok(Vec::new());
        };

        if size.is_empty() {
            return Ok(Vec::new());
        }

        let Some(mut frame) = surface.acquire().map_err(DefaultRuntimeError::Acquire)? else {
            return Ok(Vec::new());
        };

        let plan = project_ucr(ucr);
        renderer
            .render(&mut frame, &plan)
            .map_err(DefaultRuntimeError::Render)?;
        frame.present().map_err(DefaultRuntimeError::Present)?;

        Ok(Vec::new())
    }
}

impl<T, R> PlatformHandler<T> for DefaultRuntime<T, R>
where
    T: SurfaceTarget,
    R: Renderer<T>,
    R::Error: Error + 'static,
    <R::Surface as Surface>::Error: Error + 'static,
{
    type Error = DefaultRuntimeError<R::Error, <R::Surface as Surface>::Error>;

    fn handle_event(
        &mut self,
        event: PlatformEvent<T>,
    ) -> Result<Vec<PlatformCommand>, Self::Error> {
        match event {
            PlatformEvent::Resumed => Ok(self.handle_resumed()),
            PlatformEvent::WindowCreated { target, size } => {
                self.handle_window_created(target, size)
            }
            PlatformEvent::RedrawRequested => self.handle_redraw_requested(),
            PlatformEvent::Resized { size } => self.handle_resized(size),
            PlatformEvent::CloseRequested => {
                if self.is_finished() {
                    return Ok(Vec::new());
                }

                self.state = State::Finished;
                Ok(vec![PlatformCommand::Exit])
            }
            PlatformEvent::Suspended => {
                if !self.is_finished() {
                    self.state = State::Idle;
                }

                Ok(Vec::new())
            }
        }
    }

    fn shutdown(&mut self) {
        self.state = State::Finished;
    }
}
