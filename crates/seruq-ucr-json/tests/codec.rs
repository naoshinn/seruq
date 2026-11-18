use seruq_ucr::{
    AssetPath, AssetPathError, ElementKind, FfrCall, FontSizeError, GeometryError, Number, Timing,
    ValidationError, ValidationWarning, Value, ValueMap,
};
use seruq_ucr_json::{Error, from_str, to_string, to_string_pretty};

/// Wrap one element's fields in a single-element document.
fn document(element: &str) -> String {
    format!(r#"{{"metadata":{{"version":"0.1.0"}},"root":"0","elements":{{"0":{element}}}}}"#)
}

fn element(kind: &str) -> String {
    document(&format!(
        r#"{{"position":{{"x":0,"y":0}},"size":{{"width":1,"height":1}},"kind":{kind}}}"#
    ))
}

fn assert_json_error(json: &str) {
    assert!(
        matches!(from_str(json), Err(Error::Json(_))),
        "expected a JSON error for {json}"
    );
}

const EXAMPLE: &str = r#"{
  "metadata": {
    "version": "0.1.0",
    "default_font": "fonts/default.ttf"
  },
  "root": "0",
  "elements": {
    "0": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 320.0,
        "height": 120.0
      },
      "kind": {
        "type": "container",
        "children": [
          "3",
          "1",
          "18446744073709551615",
          "2",
          "4"
        ]
      }
    },
    "1": {
      "position": {
        "x": 16.0,
        "y": -16.5
      },
      "size": {
        "width": 288.0,
        "height": 0.0
      },
      "kind": {
        "type": "text",
        "text": "Hello, \"Seruq\"\n",
        "font_size": 24.0,
        "font": "fonts/title.ttf"
      }
    },
    "2": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 1.0,
        "height": 1.0
      },
      "kind": {
        "type": "image",
        "source": "images/logo.png"
      }
    },
    "3": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 1.0,
        "height": 1.0
      },
      "kind": {
        "type": "button",
        "label": "OK",
        "font_size": 0.1
      }
    },
    "4": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 1.0,
        "height": 1.0
      },
      "kind": {
        "type": "text_input",
        "value": "",
        "font_size": 16.0,
        "placeholder": "Name"
      }
    },
    "18446744073709551615": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 1.0,
        "height": 1.0
      },
      "kind": {
        "type": "geometry"
      }
    }
  }
}"#;

#[test]
fn decodes_every_element_type() {
    let draft = from_str(EXAMPLE).unwrap();
    assert_eq!(draft.metadata.version(), "0.1.0");
    assert_eq!(
        draft.metadata.default_font(),
        Some(&AssetPath::new("fonts/default.ttf").unwrap())
    );
    assert_eq!(draft.root, 0);
    assert_eq!(draft.elements.len(), 6);
    assert_eq!(
        draft.elements[&0].kind,
        ElementKind::Container {
            children: vec![3, 1, u64::MAX, 2, 4]
        }
    );
    let text = &draft.elements[&1];
    assert_eq!((text.position.x(), text.position.y()), (16.0, -16.5));
    assert_eq!((text.size.width(), text.size.height()), (288.0, 0.0));
    let ElementKind::Text {
        text,
        font_size,
        font,
    } = &text.kind
    else {
        panic!("element 1 is not Text");
    };
    assert_eq!(text, "Hello, \"Seruq\"\n");
    assert_eq!(font_size.get(), 24.0);
    assert_eq!(font.as_ref().unwrap().as_str(), "fonts/title.ttf");
    assert_eq!(draft.elements[&u64::MAX].kind, ElementKind::Geometry);
    assert!(draft.validate().is_valid());
}

#[test]
fn pretty_output_is_stable_and_sorted_by_numeric_id() {
    let draft = from_str(EXAMPLE).unwrap();
    assert_eq!(to_string_pretty(&draft).unwrap(), EXAMPLE);
}

#[test]
fn compact_output_round_trips() {
    let draft = from_str(EXAMPLE).unwrap();
    let json = to_string(&draft).unwrap();
    assert!(!json.contains('\n'));
    assert_eq!(from_str(&json).unwrap(), draft);
}

#[test]
fn round_trips_f32_extremes() {
    for value in [f32::MIN_POSITIVE, f32::MAX, 0.1, 16_777_217.0] {
        let json = element(&format!(
            r#"{{"type":"text","text":"","font_size":{value:?}}}"#
        ));
        let draft = from_str(&json).unwrap();
        let decoded = from_str(&to_string(&draft).unwrap()).unwrap();
        let ElementKind::Text { font_size, .. } = &decoded.elements[&0].kind else {
            panic!("element 0 is not Text");
        };
        assert_eq!(font_size.get(), value);
    }
}

#[test]
fn accepts_integer_numbers_and_null_optionals() {
    let draft = from_str(&element(
        r#"{"type":"text_input","value":"v","font_size":16,"font":null,"placeholder":null}"#,
    ))
    .unwrap();
    assert_eq!(
        draft.elements[&0].kind,
        from_str(&element(
            r#"{"type":"text_input","value":"v","font_size":16.0}"#
        ))
        .unwrap()
        .elements[&0]
            .kind
    );
}

#[test]
fn decoding_does_not_validate_structure() {
    let json = r#"{"metadata":{"version":"0.1.0"},"root":"0","elements":{
        "0":{"position":{"x":0,"y":0},"size":{"width":1,"height":1},"kind":{"type":"container","children":["1","1","7"]}},
        "1":{"position":{"x":0,"y":0},"size":{"width":1,"height":1},"kind":{"type":"container","children":["1"]}},
        "5":{"position":{"x":0,"y":0},"size":{"width":1,"height":1},"kind":{"type":"geometry"}}
    }}"#;
    let draft = from_str(json).unwrap();
    assert_eq!(from_str(&to_string(&draft).unwrap()).unwrap(), draft);

    let report = draft.validate();
    assert_eq!(
        report.errors(),
        [
            ValidationError::DuplicateReference {
                parent: 0,
                child: 1
            },
            ValidationError::ElementNotFound {
                parent: 0,
                child: 7
            },
            ValidationError::Cycle {
                parent: 1,
                child: 1
            },
        ]
    );
    assert_eq!(
        report.warnings(),
        [ValidationWarning::UnusedElement { id: 5 }]
    );
}

#[test]
fn rejects_non_canonical_ids() {
    for id in [
        r#""00""#,
        r#""01""#,
        r#""+1""#,
        r#""-1""#,
        r#"" 1""#,
        r#""1e3""#,
        r#""1.0""#,
        r#""""#,
        r#""18446744073709551616""#,
        "1",
    ] {
        assert_json_error(&format!(
            r#"{{"metadata":{{"version":"0.1.0"}},"root":{id},"elements":{{}}}}"#
        ));
        assert_json_error(&element(&format!(
            r#"{{"type":"container","children":[{id}]}}"#
        )));
    }
    assert_json_error(&document("{}").replace(r#""0":{"#, r#""01":{"#));
}

#[test]
fn rejects_duplicate_element_ids() {
    let element =
        r#"{"position":{"x":0,"y":0},"size":{"width":1,"height":1},"kind":{"type":"geometry"}}"#;
    let json = format!(
        r#"{{"metadata":{{"version":"0.1.0"}},"root":"0","elements":{{"0":{element},"0":{element}}}}}"#
    );
    let error = from_str(&json).unwrap_err();
    assert!(error.to_string().contains("duplicate element ID 0"));
}

#[test]
fn rejects_unknown_missing_and_duplicate_fields() {
    for kind in [
        r#"{"type":"window"}"#,
        r#"{"type":"Text","text":"","font_size":1}"#,
        r#"{"text":"","font_size":1}"#,
        r#"{"type":"text","text":"","font_size":1,"color":"red"}"#,
        r#"{"type":"text","text":""}"#,
        r#"{"type":"text","text":"","text":"","font_size":1}"#,
        r#"{"type":"container"}"#,
        r#"{"type":"container","children":null}"#,
        r#"{"type":"geometry","shape":{}}"#,
        r#"{"type":"text","text":"\ud800","font_size":1}"#,
    ] {
        assert_json_error(&element(kind));
    }
    assert_json_error(&document(
        r#"{"position":{"x":0,"y":0,"z":0},"size":{"width":1,"height":1},"kind":{"type":"geometry"}}"#,
    ));
    assert_json_error(&document(
        r#"{"position":{"x":0,"y":0},"kind":{"type":"geometry"}}"#,
    ));
    assert_json_error(r#"{"root":"0","elements":{}}"#);
    assert_json_error(r#"{"metadata":{"version":"0.1.0"},"root":"0","elements":{},"extra":1}"#);
    assert_json_error(r#"{"metadata":{"version":"0.1.0"},"root":"0","elements":{}} 1"#);
}

#[test]
fn rejects_numbers_outside_finite_f32() {
    for number in [
        "1e39",
        "-1e39",
        r#""NaN""#,
        r#""inf""#,
        r#""-inf""#,
        r#""16""#,
        "null",
    ] {
        assert_json_error(&element(&format!(
            r#"{{"type":"text","text":"","font_size":{number}}}"#
        )));
        assert_json_error(&document(&format!(
            r#"{{"position":{{"x":{number},"y":0}},"size":{{"width":1,"height":1}},"kind":{{"type":"geometry"}}}}"#
        )));
    }
}

#[test]
fn reports_element_for_invalid_size() {
    let json = document(
        r#"{"position":{"x":0,"y":0},"size":{"width":-1,"height":1},"kind":{"type":"geometry"}}"#,
    );
    assert!(matches!(
        from_str(&json),
        Err(Error::InvalidGeometry {
            element: 0,
            field: "size",
            reason: GeometryError::Negative,
        })
    ));
}

#[test]
fn reports_element_for_invalid_font_size() {
    for (kind, font_size) in [("text", "0"), ("button", "-1"), ("text_input", "-0.0")] {
        let field = match kind {
            "text" => "text",
            "button" => "label",
            _ => "value",
        };
        let json = element(&format!(
            r#"{{"type":"{kind}","{field}":"","font_size":{font_size}}}"#
        ));
        assert!(
            matches!(
                from_str(&json),
                Err(Error::InvalidFontSize {
                    element: 0,
                    reason: FontSizeError::NonPositive,
                })
            ),
            "expected InvalidFontSize for {kind} with {font_size}"
        );
    }
}

#[test]
fn reports_location_of_invalid_asset_paths() {
    assert!(matches!(
        from_str(&element(r#"{"type":"image","source":"../secret.png"}"#)),
        Err(Error::InvalidAssetPath {
            element: Some(0),
            field: "source",
            reason: AssetPathError::InvalidSegment,
        })
    ));
    assert!(matches!(
        from_str(&element(
            r#"{"type":"button","label":"","font_size":1,"font":"/usr/share/fonts/a.ttf"}"#
        )),
        Err(Error::InvalidAssetPath {
            element: Some(0),
            field: "font",
            reason: AssetPathError::Absolute,
        })
    ));
    assert!(matches!(
        from_str(r#"{"metadata":{"version":"0.1.0","default_font":""},"root":"0","elements":{}}"#),
        Err(Error::InvalidAssetPath {
            element: None,
            field: "default_font",
            reason: AssetPathError::Empty,
        })
    ));
}

const STATEFUL: &str = r#"{
  "metadata": {
    "version": "0.1.0"
  },
  "root": "0",
  "elements": {
    "0": {
      "position": {
        "x": 0.0,
        "y": 0.0
      },
      "size": {
        "width": 1.0,
        "height": 1.0
      },
      "kind": {
        "type": "geometry"
      },
      "data": {
        "done": false,
        "item_id": 1.0
      }
    }
  },
  "state": {
    "": "empty key",
    "filter": "all",
    "nested": {
      "list": [
        true,
        -0.5,
        "text",
        [],
        {}
      ]
    },
    "next_id": 3.0
  }
}"#;

#[test]
fn state_and_data_round_trip() {
    let draft = from_str(STATEFUL).unwrap();
    assert_eq!(draft.state["next_id"], Value::from(3));
    assert_eq!(draft.state["filter"], Value::from("all"));
    assert_eq!(
        draft.state["nested"].as_map().unwrap()["list"],
        Value::from(vec![
            Value::from(true),
            Value::from(Number::new(-0.5).unwrap()),
            Value::from("text"),
            Value::from(Vec::new()),
            Value::from(ValueMap::new()),
        ])
    );
    assert_eq!(draft.elements[&0].data["done"], Value::from(false));
    assert_eq!(draft.elements[&0].data["item_id"], Value::from(1));
    assert_eq!(to_string_pretty(&draft).unwrap(), STATEFUL);
    assert_eq!(from_str(&to_string(&draft).unwrap()).unwrap(), draft);
}

#[test]
fn absent_and_empty_state_and_data_are_equivalent() {
    let absent = from_str(&element(r#"{"type":"geometry"}"#)).unwrap();
    assert!(absent.state.is_empty());
    assert!(absent.elements[&0].data.is_empty());

    let empty = r#"{"metadata":{"version":"0.1.0"},"root":"0","state":{},"elements":{"0":
        {"position":{"x":0,"y":0},"size":{"width":1,"height":1},"kind":{"type":"geometry"},"data":{}}}}"#;
    assert_eq!(from_str(empty).unwrap(), absent);

    let json = to_string(&absent).unwrap();
    assert!(!json.contains(r#""state""#) && !json.contains(r#""data""#));
}

#[test]
fn state_numbers_use_f64() {
    let json = r#"{"metadata":{"version":"0.1.0"},"root":"0","elements":{},
        "state":{"a":0.1,"b":9007199254740993,"c":-1e308,"d":1}"#
        .to_owned()
        + "}";
    let draft = from_str(&json).unwrap();
    assert_eq!(draft.state["a"].as_number(), Some(0.1));
    assert_eq!(draft.state["b"].as_number(), Some(9_007_199_254_740_992.0));
    assert_eq!(draft.state["c"].as_number(), Some(-1e308));
    assert_eq!(draft.state["d"].as_number(), Some(1.0));
    assert_eq!(from_str(&to_string(&draft).unwrap()).unwrap(), draft);
}

#[test]
fn rejects_invalid_state_and_data_values() {
    let state = |state: &str| {
        format!(
            r#"{{"metadata":{{"version":"0.1.0"}},"root":"0","elements":{{}},"state":{state}}}"#
        )
    };
    for value in [
        r#"{"a":null}"#,
        r#"{"a":[null]}"#,
        r#"{"a":{"b":null}}"#,
        r#"{"a":1e999}"#,
        r#"{"a":1,"a":2}"#,
        r#"{"a":{"b":1,"b":1}}"#,
        r#"{"a":[{"b":1,"b":1}]}"#,
        "null",
        "[]",
        r#""text""#,
    ] {
        assert_json_error(&state(value));
        assert_json_error(&document(&format!(
            r#"{{"position":{{"x":0,"y":0}},"size":{{"width":1,"height":1}},"kind":{{"type":"geometry"}},"data":{value}}}"#
        )));
    }
}

#[test]
fn actions_round_trip_in_timing_order() {
    let json = element(r#"{"type":"button","label":"","font_size":1}"#).replace(
        r#""kind""#,
        r#""actions":{"value_changed":{"function_name":"f"},"unfocused":{"function_name":"e"},
            "focused":{"function_name":"d"},"activated":{"function_name":"app.confirm"},
            "released":{"function_name":"b"},"pushed":{"function_name":""}},"kind""#,
    );
    let draft = from_str(&json).unwrap();
    let actions = &draft.elements[&0].actions;
    assert_eq!(
        actions.keys().copied().collect::<Vec<_>>(),
        [
            Timing::Pushed,
            Timing::Released,
            Timing::Activated,
            Timing::Focused,
            Timing::Unfocused,
            Timing::ValueChanged,
        ]
    );
    assert_eq!(actions[&Timing::Activated], FfrCall::new("app.confirm"));

    let encoded = to_string(&draft).unwrap();
    assert!(encoded.contains(
        r#""actions":{"pushed":{"function_name":""},"released":{"function_name":"b"},"activated":{"function_name":"app.confirm"},"#
    ));
    assert_eq!(from_str(&encoded).unwrap(), draft);
}

#[test]
fn absent_and_empty_actions_are_equivalent() {
    let absent = from_str(&element(r#"{"type":"geometry"}"#)).unwrap();
    let empty =
        from_str(&element(r#"{"type":"geometry"}"#).replace(r#""kind""#, r#""actions":{},"kind""#))
            .unwrap();
    assert_eq!(empty, absent);
    assert!(!to_string(&absent).unwrap().contains(r#""actions""#));
}

#[test]
fn rejects_invalid_actions() {
    for actions in [
        r#"{"clicked":{"function_name":"f"}}"#,
        r#"{"Activated":{"function_name":"f"}}"#,
        r#"{"activated":{"function_name":"f"},"activated":{"function_name":"g"}}"#,
        r#"{"activated":{}}"#,
        r#"{"activated":{"function_name":"f","arguments":[]}}"#,
        r#"{"activated":{"function_name":1}}"#,
        r#"{"activated":"f"}"#,
        r#"{"activated":null}"#,
        "null",
        "[]",
    ] {
        assert_json_error(
            &element(r#"{"type":"geometry"}"#)
                .replace(r#""kind""#, &format!(r#""actions":{actions},"kind""#)),
        );
    }
}
