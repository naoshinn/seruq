use crate::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Ucr {
    root: Node,
}

impl Ucr {
    pub fn rect(bounds: Rect, color: Color) -> Self {
        Self {
            root: Node::Rect(RectNode { bounds, color }),
        }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Rect(RectNode),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectNode {
    bounds: Rect,
    color: Color,
}

impl RectNode {
    pub fn bounds(&self) -> &Rect {
        &self.bounds
    }

    pub fn color(&self) -> &Color {
        &self.color
    }
}
