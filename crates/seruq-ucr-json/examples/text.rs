use std::collections::HashMap;

use seruq_ucr::{Element, ElementKind, Metadata, Point, Size, UcrDraft};

fn main() {
    let ucr = UcrDraft {
        metadata: Metadata::new("Example".to_string(), None),
        root: 0,
        elements: {
            let mut elements = HashMap::new();
            let _ = elements.insert(
                0,
                Element {
                    position: Point::new(100.0, 200.0).unwrap(),
                    size: Size::new(300.0, 400.0).unwrap(),
                    kind: ElementKind::Text {
                        text: "Hello!".to_string(),
                        font_size: 11.0,
                        font: None,
                    },
                },
            );
            elements
        },
    };

    let json = seruq_ucr_json::to_string_pretty(&ucr).unwrap();
    println!("{json}");
}
