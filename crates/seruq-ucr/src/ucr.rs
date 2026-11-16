use std::collections::HashMap;

use crate::{AssetPath, FontSize, Point, Size, ValueMap};

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
}

impl Metadata {
    pub fn new(version: String, default_font: Option<AssetPath>) -> Self {
        Self {
            version,
            default_font,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn default_font(&self) -> Option<&AssetPath> {
        self.default_font.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub position: Point,
    pub size: Size,
    pub kind: ElementKind,
    /// Application data that lives and dies with this element.
    pub data: ValueMap,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    Container {
        children: Vec<ElementId>,
    },
    Text {
        text: String,
        font_size: FontSize,
        font: Option<AssetPath>,
    },
    Image {
        source: AssetPath,
    },
    Button {
        label: String,
        font_size: FontSize,
        font: Option<AssetPath>,
    },
    TextInput {
        value: String,
        font_size: FontSize,
        font: Option<AssetPath>,
        placeholder: Option<String>,
    },
    Geometry,
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
