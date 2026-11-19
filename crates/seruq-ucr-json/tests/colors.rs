use seruq_ucr::{Color, ElementKind};
use seruq_ucr_json::{from_str, to_string};

fn document(kind: &str) -> String {
    format!(
        r##"{{"metadata":{{"version":"0.1.0","background":"#ABCDEF"}},"root":"0","elements":{{"0":{{"position":{{"x":0,"y":0}},"size":{{"width":80,"height":40}},"kind":{kind}}}}}}}"##
    )
}

#[test]
fn colors_round_trip_for_every_supported_element_and_viewport() {
    for kind in [
        r##"{"type":"container","children":[],"background":"#123456"}"##,
        r##"{"type":"text","text":"X","font_size":16,"color":"#123456"}"##,
        r##"{"type":"button","label":"X","font_size":16,"color":"#123456","background":"#FEDCBA"}"##,
        r##"{"type":"text_input","value":"X","font_size":16,"color":"#123456","background":"#FEDCBA"}"##,
    ] {
        let draft = from_str(&document(kind)).unwrap();
        assert_eq!(draft.metadata.background(), Some(Color::rgb(171, 205, 239)));
        let expected = Some(Color::rgb(18, 52, 86));
        match &draft.elements[&0].kind {
            ElementKind::Container { background, .. } => assert_eq!(*background, expected),
            ElementKind::Text { color, .. } => assert_eq!(*color, expected),
            ElementKind::Button {
                color, background, ..
            }
            | ElementKind::TextInput {
                color, background, ..
            } => {
                assert_eq!(*color, expected);
                assert_eq!(*background, Some(Color::rgb(254, 220, 186)));
            }
            _ => panic!("unexpected element"),
        }
        let encoded = to_string(&draft).unwrap();
        assert!(encoded.contains("#abcdef"));
        assert!(!encoded.contains("#FEDCBA"));
        assert_eq!(from_str(&encoded).unwrap(), draft);
    }
}

#[test]
fn null_colors_are_absent_and_omitted_when_written() {
    let json =
        document(r#"{"type":"button","label":"","font_size":16,"color":null,"background":null}"#)
            .replace("\"#ABCDEF\"", "null");
    let draft = from_str(&json).unwrap();
    let encoded = to_string(&draft).unwrap();
    assert!(!encoded.contains("color"));
    assert!(!encoded.contains("background"));
    assert_eq!(draft.metadata.background(), None);
}

#[test]
fn invalid_color_representations_are_rejected() {
    for value in [
        r#""red""#,
        r##""#fff""##,
        r##""#11223344""##,
        r##""#gg0000""##,
        r##""#é0000""##,
        "123",
        "[]",
        "{}",
        "true",
    ] {
        for kind in [
            format!(r#"{{"type":"container","children":[],"background":{value}}}"#),
            format!(r#"{{"type":"text","text":"","font_size":16,"color":{value}}}"#),
            format!(r#"{{"type":"button","label":"","font_size":16,"background":{value}}}"#),
            format!(r#"{{"type":"text_input","value":"","font_size":16,"color":{value}}}"#),
        ] {
            assert!(from_str(&document(&kind)).is_err(), "accepted {kind}");
        }
        let json = document(r#"{"type":"container","children":[]}"#).replace("\"#ABCDEF\"", value);
        assert!(from_str(&json).is_err(), "accepted {json}");
    }
}

#[test]
fn duplicate_color_fields_are_rejected() {
    for kind in [
        r##"{"type":"text","text":"","font_size":16,"color":"#123456","color":null}"##,
        r##"{"type":"button","label":"","font_size":16,"background":null,"background":"#123456"}"##,
    ] {
        assert!(from_str(&document(kind)).is_err());
    }
}
