use std::collections::HashMap;

use seruq_ucr::{Element, ElementKind, FontSize, Metadata, Point, Size, UcrDraft, ValueMap};

fn main() {
    let ucr = UcrDraft {
        metadata: Metadata::new("Example".to_string(), None),
        root: 0,
        elements: {
            let mut elements = HashMap::new();
            let _ = elements.insert(
                0,
                Element::new(
                    Point::new(100.0, 200.0).unwrap(),
                    Size::new(300.0, 400.0).unwrap(),
                    ElementKind::Text {
                        text: "Hello!".to_string(),
                        font_size: FontSize::new(11.0).unwrap(),
                        font: None,
                    },
                ),
            );
            elements
        },
        state: ValueMap::new(),
    };

    let json = seruq_ucr_json::to_string_pretty(&ucr).unwrap();
    println!("{json}");
}
