use super::{Element, ElementId, ElementKind, Metadata, UcrDraft};
use crate::validate::{ValidationError, ValidationWarning};
use crate::{Point, Size};

fn draft(
    root: ElementId,
    elements: impl IntoIterator<Item = (ElementId, ElementKind)>,
) -> UcrDraft {
    UcrDraft {
        metadata: Metadata::new("0.1.0".into(), Some("fonts/default.ttf".into())),
        root,
        elements: elements
            .into_iter()
            .map(|(id, kind)| {
                (
                    id,
                    Element {
                        position: Point::new(0.0, 0.0).unwrap(),
                        size: Size::new(100.0, 100.0).unwrap(),
                        kind,
                    },
                )
            })
            .collect(),
    }
}

fn container(children: &[ElementId]) -> ElementKind {
    ElementKind::Container {
        children: children.to_vec(),
    }
}

fn text(font_size: f32) -> ElementKind {
    ElementKind::Text {
        text: "Text".into(),
        font_size,
        font: None,
    }
}

fn button(font_size: f32) -> ElementKind {
    ElementKind::Button {
        label: "Button".into(),
        font_size,
        font: None,
    }
}

fn text_input(font_size: f32) -> ElementKind {
    ElementKind::TextInput {
        value: "Value".into(),
        font_size,
        font: None,
        placeholder: None,
    }
}

fn assert_clean(draft: &UcrDraft) {
    let report = draft.validate();
    assert!(report.errors().is_empty(), "expected no validation errors");
    assert!(
        report.warnings().is_empty(),
        "expected no validation warnings"
    );
}

fn assert_rejected(draft: &UcrDraft) {
    let report = draft.validate();
    assert!(!report.errors().is_empty(), "expected a validation error");
}

fn assert_unused_warning(draft: &UcrDraft) {
    let report = draft.validate();
    assert!(report.errors().is_empty(), "expected no validation errors");
    assert!(
        report
            .warnings()
            .iter()
            .any(|warning| matches!(warning, ValidationWarning::UnusedElement)),
        "expected a warning for unreachable elements"
    );
}

#[test]
fn accepts_minimum_ucr() {
    assert_clean(&draft(0, [(0, container(&[]))]));
}

#[test]
fn accepts_non_container_root() {
    for kind in [
        text(16.0),
        ElementKind::Image {
            source: "images/example.png".into(),
        },
        button(16.0),
        text_input(16.0),
        ElementKind::Geometry,
    ] {
        assert_clean(&draft(0, [(0, kind)]));
    }
}

#[test]
fn accepts_valid_nested_tree() {
    assert_clean(&draft(
        1,
        [
            (30, button(16.0)),
            (10, text(16.0)),
            (1, container(&[20, 10])),
            (20, container(&[30])),
        ],
    ));
}

#[test]
fn accepts_zero_size_and_negative_position() {
    let mut draft = draft(0, [(0, text(16.0))]);
    let element = draft.elements.get_mut(&0).unwrap();
    element.position = Point::new(-10.0, -20.0).unwrap();
    element.size = Size::new(0.0, 0.0).unwrap();
    assert_clean(&draft);
}

#[test]
fn rejects_missing_root() {
    assert_rejected(&draft(0, [(1, container(&[]))]));
}

#[test]
fn rejects_empty_element_map() {
    assert_rejected(&draft(0, []));
}

#[test]
fn rejects_missing_child() {
    assert_rejected(&draft(0, [(0, container(&[1]))]));
}

#[test]
fn warns_about_unreachable_parent_of_root() {
    assert_unused_warning(&draft(0, [(0, text(16.0)), (1, container(&[0]))]));
}

#[test]
fn warns_about_unreachable_parent_of_reachable_element() {
    assert_unused_warning(&draft(
        0,
        [(0, container(&[1])), (1, text(16.0)), (2, container(&[1]))],
    ));
}

#[test]
fn rejects_duplicate_children() {
    let report = draft(0, [(0, container(&[1, 1])), (1, text(16.0))]).validate();
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::DuplicateReference))
    );
}

#[test]
fn rejects_multiple_parents() {
    let report = draft(
        0,
        [
            (0, container(&[1, 2])),
            (1, container(&[3])),
            (2, container(&[3])),
            (3, text(16.0)),
        ],
    )
    .validate();
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::DuplicateReference))
    );
}

#[test]
fn rejects_self_reference() {
    let report = draft(0, [(0, container(&[0]))]).validate();
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::Cycle))
    );
}

#[test]
fn rejects_cycle_through_root() {
    let report = draft(0, [(0, container(&[1])), (1, container(&[0]))]).validate();
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::Cycle))
    );
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::RootHasParent))
    );
}

#[test]
fn rejects_cycle_below_root() {
    let report = draft(
        0,
        [
            (0, container(&[1])),
            (1, container(&[2])),
            (2, container(&[1])),
        ],
    )
    .validate();
    assert!(
        report
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::Cycle))
    );
}

#[test]
fn warns_about_cycle_in_unreachable_elements() {
    assert_unused_warning(&draft(
        0,
        [(0, text(16.0)), (1, container(&[2])), (2, container(&[1]))],
    ));
}

#[test]
fn warns_about_missing_child_in_unreachable_container() {
    assert_unused_warning(&draft(0, [(0, text(16.0)), (1, container(&[2]))]));
}

#[test]
fn warns_about_unreachable_element_without_rejecting_ucr() {
    assert_unused_warning(&draft(0, [(0, container(&[])), (1, text(16.0))]));
}

#[test]
fn rejects_invalid_text_font_size() {
    for font_size in [
        0.0,
        -1.0,
        -f32::EPSILON,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let report = draft(0, [(0, text(font_size))]).validate();
        assert!(
            report
                .errors()
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidFontSize)),
            "expected InvalidFontSize for {font_size:?}"
        );
    }
}

#[test]
fn rejects_invalid_button_font_size() {
    for font_size in [
        0.0,
        -1.0,
        -f32::EPSILON,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let report = draft(0, [(0, button(font_size))]).validate();
        assert!(
            report
                .errors()
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidFontSize)),
            "expected InvalidFontSize for {font_size:?}"
        );
    }
}

#[test]
fn rejects_invalid_text_input_font_size() {
    for font_size in [
        0.0,
        -1.0,
        -f32::EPSILON,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let report = draft(0, [(0, text_input(font_size))]).validate();
        assert!(
            report
                .errors()
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidFontSize)),
            "expected InvalidFontSize for {font_size:?}"
        );
    }
}

#[test]
fn accepts_positive_finite_font_sizes() {
    for font_size in [f32::MIN_POSITIVE, 16.0, f32::MAX] {
        assert_clean(&draft(
            0,
            [
                (0, container(&[1, 2, 3])),
                (1, text(font_size)),
                (2, button(font_size)),
                (3, text_input(font_size)),
            ],
        ));
    }
}

#[test]
fn warns_about_invalid_font_size_in_unreachable_element() {
    for font_size in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for kind in [text(font_size), button(font_size), text_input(font_size)] {
            assert_unused_warning(&draft(0, [(0, container(&[])), (1, kind)]));
        }
    }
}

#[test]
fn rejects_invalid_font_size_in_reachable_descendant() {
    for kind in [text(0.0), button(f32::NAN), text_input(f32::INFINITY)] {
        let report = draft(0, [(0, container(&[1])), (1, kind)]).validate();
        assert!(
            report
                .errors()
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidFontSize))
        );
    }
}
