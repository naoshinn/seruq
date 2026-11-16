use super::{Element, ElementId, ElementKind, Metadata, UcrDraft};
use crate::validate::{ValidationError, ValidationWarning};
use crate::{AssetPath, FontSize, Point, Size, ValueMap};

fn draft(
    root: ElementId,
    elements: impl IntoIterator<Item = (ElementId, ElementKind)>,
) -> UcrDraft {
    UcrDraft {
        metadata: Metadata::new(
            "0.1.0".into(),
            Some(AssetPath::new("fonts/default.ttf").unwrap()),
        ),
        root,
        elements: elements
            .into_iter()
            .map(|(id, kind)| {
                (
                    id,
                    Element::new(
                        Point::new(0.0, 0.0).unwrap(),
                        Size::new(100.0, 100.0).unwrap(),
                        kind,
                    ),
                )
            })
            .collect(),
        state: ValueMap::new(),
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
        font_size: FontSize::new(font_size).unwrap(),
        font: None,
    }
}

fn button(font_size: f32) -> ElementKind {
    ElementKind::Button {
        label: "Button".into(),
        font_size: FontSize::new(font_size).unwrap(),
        font: None,
    }
}

fn text_input(font_size: f32) -> ElementKind {
    ElementKind::TextInput {
        value: "Value".into(),
        font_size: FontSize::new(font_size).unwrap(),
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

fn assert_errors(draft: &UcrDraft, expected: &[ValidationError]) {
    assert_eq!(draft.validate().errors(), expected);
}

fn assert_unused(draft: &UcrDraft, expected: &[ElementId]) {
    let report = draft.validate();
    assert!(report.errors().is_empty(), "expected no validation errors");
    let expected: Vec<_> = expected
        .iter()
        .map(|&id| ValidationWarning::UnusedElement { id })
        .collect();
    assert_eq!(report.warnings(), expected);
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
            source: AssetPath::new("images/example.png").unwrap(),
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
    assert_errors(
        &draft(0, [(1, container(&[]))]),
        &[ValidationError::RootNotFound { root: 0 }],
    );
}

#[test]
fn rejects_empty_element_map() {
    assert_errors(&draft(0, []), &[ValidationError::RootNotFound { root: 0 }]);
}

#[test]
fn rejects_missing_child() {
    assert_errors(
        &draft(0, [(0, container(&[1]))]),
        &[ValidationError::ElementNotFound {
            parent: 0,
            child: 1,
        }],
    );
}

#[test]
fn warns_about_unreachable_parent_of_root() {
    assert_unused(&draft(0, [(0, text(16.0)), (1, container(&[0]))]), &[1]);
}

#[test]
fn warns_about_unreachable_parent_of_reachable_element() {
    assert_unused(
        &draft(
            0,
            [(0, container(&[1])), (1, text(16.0)), (2, container(&[1]))],
        ),
        &[2],
    );
}

#[test]
fn rejects_duplicate_children() {
    assert_errors(
        &draft(0, [(0, container(&[1, 1])), (1, text(16.0))]),
        &[ValidationError::DuplicateReference {
            parent: 0,
            child: 1,
        }],
    );
}

#[test]
fn rejects_multiple_parents() {
    assert_errors(
        &draft(
            0,
            [
                (0, container(&[1, 2])),
                (1, container(&[3])),
                (2, container(&[3])),
                (3, text(16.0)),
            ],
        ),
        &[ValidationError::DuplicateReference {
            parent: 2,
            child: 3,
        }],
    );
}

#[test]
fn rejects_root_self_reference() {
    assert_errors(
        &draft(0, [(0, container(&[0]))]),
        &[ValidationError::RootHasParent { parent: 0 }],
    );
}

#[test]
fn rejects_self_reference_below_root() {
    assert_errors(
        &draft(0, [(0, container(&[1])), (1, container(&[1]))]),
        &[ValidationError::Cycle {
            parent: 1,
            child: 1,
        }],
    );
}

#[test]
fn reports_cycle_through_root_once() {
    assert_errors(
        &draft(0, [(0, container(&[1])), (1, container(&[0]))]),
        &[ValidationError::RootHasParent { parent: 1 }],
    );
}

#[test]
fn rejects_cycle_below_root() {
    assert_errors(
        &draft(
            0,
            [
                (0, container(&[1])),
                (1, container(&[2])),
                (2, container(&[1])),
            ],
        ),
        &[ValidationError::Cycle {
            parent: 2,
            child: 1,
        }],
    );
}

#[test]
fn reports_errors_in_traversal_order() {
    assert_errors(
        &draft(
            0,
            [
                (0, container(&[1, 2, 9])),
                (1, container(&[8])),
                (2, container(&[1])),
            ],
        ),
        &[
            ValidationError::ElementNotFound {
                parent: 0,
                child: 9,
            },
            ValidationError::ElementNotFound {
                parent: 1,
                child: 8,
            },
            ValidationError::DuplicateReference {
                parent: 2,
                child: 1,
            },
        ],
    );
}

#[test]
fn reports_every_unused_element_in_id_order() {
    assert_unused(
        &draft(
            5,
            [
                (9, text(16.0)),
                (5, container(&[])),
                (7, text(16.0)),
                (1, text(16.0)),
            ],
        ),
        &[1, 7, 9],
    );
}

#[test]
fn build_records_parents_of_reachable_elements() {
    let (ucr, warnings) = draft(
        1,
        [
            (1, container(&[20, 10])),
            (20, container(&[30])),
            (10, text(16.0)),
            (30, button(16.0)),
            (40, container(&[10])),
        ],
    )
    .build()
    .unwrap();
    assert_eq!(warnings, [ValidationWarning::UnusedElement { id: 40 }]);
    assert_eq!(ucr.parent(1), None);
    assert_eq!(ucr.parent(20), Some(1));
    assert_eq!(ucr.parent(10), Some(1));
    assert_eq!(ucr.parent(30), Some(20));
    assert_eq!(ucr.parent(40), None);
}

#[test]
fn warns_about_cycle_in_unreachable_elements() {
    assert_unused(
        &draft(
            0,
            [(0, text(16.0)), (1, container(&[2])), (2, container(&[1]))],
        ),
        &[1, 2],
    );
}

#[test]
fn warns_about_missing_child_in_unreachable_container() {
    assert_unused(&draft(0, [(0, text(16.0)), (1, container(&[2]))]), &[1]);
}

#[test]
fn warns_about_unreachable_element_without_rejecting_ucr() {
    assert_unused(&draft(0, [(0, container(&[])), (1, text(16.0))]), &[1]);
}
