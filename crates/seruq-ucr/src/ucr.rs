use std::collections::HashMap;

use crate::{Actions, AssetPath, Color, FontSize, Point, Size, ValueMap};

pub type ElementId = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct UcrDraft {
    pub metadata: Metadata,
    pub root: ElementId,
    pub elements: HashMap<ElementId, Element>,
    /// Application data that belongs to no particular element.
    pub state: ValueMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    version: String,
    default_font: Option<AssetPath>,
    background: Option<Color>,
}

impl Metadata {
    pub fn new(version: String, default_font: Option<AssetPath>) -> Self {
        Self {
            version,
            default_font,
            background: None,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn default_font(&self) -> Option<&AssetPath> {
        self.default_font.as_ref()
    }

    pub fn background(&self) -> Option<Color> {
        self.background
    }

    pub fn with_background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub position: Point,
    pub size: Size,
    pub kind: ElementKind,
    /// Functions to call through FFR when something happens to this element.
    pub actions: Actions,
    /// Application data that lives and dies with this element.
    pub data: ValueMap,
}

impl Element {
    /// An element without actions or data.
    pub fn new(position: Point, size: Size, kind: ElementKind) -> Self {
        Self {
            position,
            size,
            kind,
            actions: Actions::new(),
            data: ValueMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    Container {
        children: Vec<ElementId>,
        background: Option<Color>,
    },
    Text {
        text: String,
        font_size: FontSize,
        font: Option<AssetPath>,
        color: Option<Color>,
    },
    Image {
        source: AssetPath,
    },
    Button {
        label: String,
        font_size: FontSize,
        font: Option<AssetPath>,
        color: Option<Color>,
        background: Option<Color>,
    },
    TextInput {
        value: String,
        font_size: FontSize,
        font: Option<AssetPath>,
        placeholder: Option<String>,
        color: Option<Color>,
        background: Option<Color>,
    },
    Geometry,
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
