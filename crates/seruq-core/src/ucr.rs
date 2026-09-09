use crate::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Ucr {
    root: Node,
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
