use crate::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Ucr {
    root: Node,
}

impl Ucr {
    pub fn new(root: Node) -> Self {
        Self { root }
    }

    pub fn rect(bounds: Rect, color: Color) -> Self {
        Self::new(Node::Rect(RectNode::new(bounds, color)))
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
    pub fn new(bounds: Rect, color: Color) -> Self {
        Self { bounds, color }
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn color(&self) -> Color {
        self.color
    }
}
