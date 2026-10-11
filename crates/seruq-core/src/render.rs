use std::{error::Error, fmt};

use crate::{Color, Node, PhysicalSize, Rect, Surface, SurfaceTarget, Ucr};

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPlan {
    commands: Vec<DrawCommand>,
}

impl RenderPlan {
    pub fn new(commands: Vec<DrawCommand>) -> Self {
        Self { commands }
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    FillRect { bounds: Rect, color: Color },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSurfaceError<T, E> {
    target: T,
    error: E,
}

impl<T, E> CreateSurfaceError<T, E> {
    pub fn new(target: T, error: E) -> Self {
        Self { target, error }
    }

    pub fn target(&self) -> &T {
        &self.target
    }

    pub fn error(&self) -> &E {
        &self.error
    }

    pub fn into_parts(self) -> (T, E) {
        (self.target, self.error)
    }
}

impl<T, E: fmt::Display> fmt::Display for CreateSurfaceError<T, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, formatter)
    }
}

impl<T: fmt::Debug, E: Error + 'static> Error for CreateSurfaceError<T, E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

pub trait Renderer<T: SurfaceTarget> {
    type Surface: Surface;
    type Error;

    fn create_surface(
        &mut self,
        target: T,
        size: PhysicalSize,
    ) -> Result<Self::Surface, CreateSurfaceError<T, Self::Error>>;

    fn render(
        &mut self,
        frame: &mut <Self::Surface as Surface>::Frame<'_>,
        plan: &RenderPlan,
    ) -> Result<(), Self::Error>;
}

pub fn project_ucr(ucr: &Ucr) -> RenderPlan {
    let mut commands = Vec::new();

    match ucr.root() {
        Node::Rect(node) => {
            if !node.bounds().is_empty() {
                commands.push(DrawCommand::FillRect {
                    bounds: node.bounds(),
                    color: node.color(),
                });
            }
        }
    }

    RenderPlan::new(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color() -> Color {
        Color::new(0.1, 0.2, 0.3).unwrap()
    }

    fn origin() -> crate::Point {
        crate::Point::new(10.0, 20.0).unwrap()
    }

    #[test]
    fn projects_rect_ucr_to_fill_rect() {
        let size = crate::Size::new(30.0, 40.0).unwrap();
        let bounds = Rect::new(origin(), size);
        let color = color();
        let ucr = Ucr::rect(bounds, color);

        assert_eq!(
            project_ucr(&ucr),
            RenderPlan::new(vec![DrawCommand::FillRect { bounds, color }])
        );
    }

    #[test]
    fn skips_empty_rects() {
        for size in [
            crate::Size::new(0.0, 40.0).unwrap(),
            crate::Size::new(30.0, 0.0).unwrap(),
            crate::Size::new(0.0, 0.0).unwrap(),
        ] {
            let ucr = Ucr::rect(Rect::new(origin(), size), color());

            assert!(project_ucr(&ucr).is_empty());
        }
    }
}
