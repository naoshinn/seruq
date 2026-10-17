use std::collections::HashMap;

use crate::{Point, Size};

pub type ElementId = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct UcrDraft {
    pub metadata: Metadata,
    pub root: ElementId,
    pub elements: HashMap<ElementId, Element>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    version: String,
    default_font: Option<String>,
}

impl Metadata {
    pub fn new(version: String, default_font: Option<String>) -> Self {
        Self {
            version,
            default_font,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn default_font(&self) -> Option<&str> {
        self.default_font.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub position: Point,
    pub size: Size,
    pub kind: ElementKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    Container {
        children: Vec<ElementId>,
    },
    Text {
        text: String,
        font_size: f32,
        font: Option<String>,
    },
    Image {
        source: String,
    },
    Button {
        label: String,
        font_size: f32,
        font: Option<String>,
    },
    TextInput {
        value: String,
        font_size: f32,
        font: Option<String>,
        placeholder: Option<String>,
    },

    Geometry,
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
