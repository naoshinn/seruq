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
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    FillRect { bounds: Rect, color: Color },
}

pub trait Renderer<T: SurfaceTarget> {
    type Surface: Surface;
    type Error;

    fn create_surface(
        &mut self,
        target: T,
        size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error>;

    fn render(
        &mut self,
        frame: &mut <Self::Surface as Surface>::Frame<'_>,
        plan: &RenderPlan,
    ) -> Result<(), Self::Error>;
}

pub fn project_ucr(ucr: &Ucr) -> RenderPlan {
    let command = match ucr.root() {
        Node::Rect(node) => DrawCommand::FillRect {
            bounds: *node.bounds(),
            color: *node.color(),
        },
    };

    RenderPlan::new(vec![command])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_rect_ucr_to_fill_rect() {
        let origin = crate::Point::new(10.0, 20.0).unwrap();
        let size = crate::Size::new(30.0, 40.0).unwrap();
        let bounds = Rect::new(origin, size);
        let color = Color::new(0.1, 0.2, 0.3).unwrap();
        let ucr = Ucr::rect(bounds, color);

        assert_eq!(
            project_ucr(&ucr),
            RenderPlan::new(vec![DrawCommand::FillRect { bounds, color }])
        );
    }
}
