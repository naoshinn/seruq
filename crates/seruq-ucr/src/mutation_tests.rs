use super::{MutationError, MutationErrorReason, MutationOp};
use crate::{Element, ElementId, ElementKind, FontSize, Metadata, Point, Size, Ucr, UcrDraft};

fn element(kind: ElementKind) -> Element {
    Element {
        position: Point::new(0.0, 0.0).unwrap(),
        size: Size::new(100.0, 100.0).unwrap(),
        kind,
    }
}

fn container(children: &[ElementId]) -> ElementKind {
    ElementKind::Container {
        children: children.to_vec(),
    }
}

fn text(text: &str) -> ElementKind {
    ElementKind::Text {
        text: text.into(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

fn button(label: &str) -> ElementKind {
    ElementKind::Button {
        label: label.into(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

/// 0: [1, 2], 1: [3, 4], 2..=4: Text, 9: unused Text
fn ucr() -> Ucr {
    UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: 0,
        elements: [
            (0, container(&[1, 2])),
            (1, container(&[3, 4])),
            (2, text("2")),
            (3, text("3")),
            (4, text("4")),
            (9, text("9")),
        ]
        .into_iter()
        .map(|(id, kind)| (id, element(kind)))
        .collect(),
    }
    .build()
    .unwrap()
    .0
}

fn children(ucr: &Ucr, id: ElementId) -> &[ElementId] {
    match &ucr.data().elements[&id].kind {
        ElementKind::Container { children } => children,
        _ => panic!("element {id} is not a Container"),
    }
}

fn insert(id: ElementId, kind: ElementKind, parent: ElementId, index: usize) -> MutationOp {
    MutationOp::Insert {
        id,
        element: element(kind),
        parent,
        index,
    }
}

fn update_kind(id: ElementId, kind: ElementKind) -> MutationOp {
    MutationOp::Update {
        id,
        position: None,
        size: None,
        kind: Some(kind),
    }
}

/// The mutated document must equal the result of validating its data from scratch.
fn assert_consistent(ucr: &Ucr) {
    let (rebuilt, _) = ucr.data().clone().build().unwrap();
    assert_eq!(&rebuilt, ucr);
}

fn assert_rejected(mutation: &[MutationOp], index: usize, reason: MutationErrorReason) {
    let mut ucr = ucr();
    let before = ucr.clone();
    assert_eq!(ucr.apply(mutation), Err(MutationError { index, reason }));
    assert_eq!(
        ucr, before,
        "a failed mutation must not change the document"
    );
}

#[test]
fn empty_mutation_changes_nothing() {
    let mut ucr = ucr();
    let before = ucr.clone();
    let applied = ucr.apply(&[]).unwrap();
    assert!(applied.inserted().is_empty());
    assert!(applied.removed().is_empty());
    assert!(applied.updated().is_empty());
    assert_eq!(ucr, before);
}

#[test]
fn insert_places_element_at_index() {
    let mut ucr = ucr();
    let applied = ucr
        .apply(&[
            insert(5, text("5"), 1, 1),
            insert(6, text("6"), 1, 0),
            insert(7, text("7"), 1, 4),
        ])
        .unwrap();
    assert_eq!(children(&ucr, 1), [6, 3, 5, 4, 7]);
    assert_eq!(ucr.parent(5), Some(1));
    assert_eq!(
        applied.inserted().iter().copied().collect::<Vec<_>>(),
        [5, 6, 7]
    );
    assert!(applied.updated().is_empty());
    assert_consistent(&ucr);
}

#[test]
fn insert_builds_subtree_within_one_mutation() {
    let mut ucr = ucr();
    ucr.apply(&[
        insert(5, container(&[]), 0, 2),
        insert(6, button("6"), 5, 0),
    ])
    .unwrap();
    assert_eq!(children(&ucr, 0), [1, 2, 5]);
    assert_eq!(children(&ucr, 5), [6]);
    assert_consistent(&ucr);
}

#[test]
fn insert_rejects_invalid_targets() {
    assert_rejected(
        &[insert(3, text("3"), 0, 0)],
        0,
        MutationErrorReason::DuplicateId { id: 3 },
    );
    // Unused elements still occupy their IDs.
    assert_rejected(
        &[insert(9, text("9"), 0, 0)],
        0,
        MutationErrorReason::DuplicateId { id: 9 },
    );
    assert_rejected(
        &[insert(5, text("5"), 8, 0)],
        0,
        MutationErrorReason::ElementNotFound { id: 8 },
    );
    assert_rejected(
        &[insert(5, text("5"), 9, 0)],
        0,
        MutationErrorReason::Unreachable { id: 9 },
    );
    assert_rejected(
        &[insert(5, text("5"), 2, 0)],
        0,
        MutationErrorReason::NotContainer { id: 2 },
    );
    assert_rejected(
        &[insert(5, text("5"), 1, 3)],
        0,
        MutationErrorReason::IndexOutOfBounds {
            parent: 1,
            index: 3,
            len: 2,
        },
    );
    assert_rejected(
        &[insert(5, container(&[2]), 0, 0)],
        0,
        MutationErrorReason::InsertedContainerHasChildren { id: 5 },
    );
}

#[test]
fn remove_deletes_subtree() {
    let mut ucr = ucr();
    let applied = ucr.apply(&[MutationOp::Remove { id: 1 }]).unwrap();
    assert_eq!(children(&ucr, 0), [2]);
    for id in [1, 3, 4] {
        assert!(!ucr.data().elements.contains_key(&id));
        assert_eq!(ucr.parent(id), None);
    }
    assert_eq!(
        applied.removed().iter().copied().collect::<Vec<_>>(),
        [1, 3, 4]
    );
    assert_consistent(&ucr);
}

#[test]
fn remove_rejects_invalid_targets() {
    assert_rejected(
        &[MutationOp::Remove { id: 0 }],
        0,
        MutationErrorReason::TargetsRoot,
    );
    assert_rejected(
        &[MutationOp::Remove { id: 8 }],
        0,
        MutationErrorReason::ElementNotFound { id: 8 },
    );
    assert_rejected(
        &[MutationOp::Remove { id: 9 }],
        0,
        MutationErrorReason::Unreachable { id: 9 },
    );
    assert_rejected(
        &[MutationOp::Remove { id: 1 }, MutationOp::Remove { id: 3 }],
        1,
        MutationErrorReason::ElementNotFound { id: 3 },
    );
}

#[test]
fn removed_id_can_be_reused() {
    let mut ucr = ucr();
    let applied = ucr
        .apply(&[MutationOp::Remove { id: 2 }, insert(2, button("2"), 1, 0)])
        .unwrap();
    assert_eq!(children(&ucr, 0), [1]);
    assert_eq!(children(&ucr, 1), [2, 3, 4]);
    assert!(applied.removed().contains(&2));
    assert!(applied.inserted().contains(&2));
    assert_consistent(&ucr);
}

#[test]
fn inserting_then_removing_reports_nothing() {
    let mut ucr = ucr();
    let before = ucr.clone();
    let applied = ucr
        .apply(&[
            insert(5, container(&[]), 0, 0),
            insert(6, text("6"), 5, 0),
            MutationOp::Remove { id: 5 },
        ])
        .unwrap();
    assert!(applied.inserted().is_empty());
    assert!(applied.removed().is_empty());
    assert_eq!(ucr, before);
}

#[test]
fn move_reparents_element() {
    let mut ucr = ucr();
    let applied = ucr
        .apply(&[MutationOp::Move {
            id: 3,
            parent: 0,
            index: 1,
        }])
        .unwrap();
    assert_eq!(children(&ucr, 0), [1, 3, 2]);
    assert_eq!(children(&ucr, 1), [4]);
    assert_eq!(ucr.parent(3), Some(0));
    assert_eq!(applied.updated().iter().copied().collect::<Vec<_>>(), [3]);
    assert_consistent(&ucr);
}

#[test]
fn move_reorders_within_same_parent() {
    let mut ucr = ucr();
    ucr.apply(&[MutationOp::Move {
        id: 3,
        parent: 1,
        index: 1,
    }])
    .unwrap();
    assert_eq!(children(&ucr, 1), [4, 3]);
    assert_consistent(&ucr);

    // The index counts positions after detaching, so 2 is out of range here.
    assert_rejected(
        &[MutationOp::Move {
            id: 3,
            parent: 1,
            index: 2,
        }],
        0,
        MutationErrorReason::IndexOutOfBounds {
            parent: 1,
            index: 2,
            len: 1,
        },
    );
}

#[test]
fn move_keeps_subtree_attached() {
    let mut ucr = ucr();
    ucr.apply(&[
        insert(5, container(&[]), 0, 0),
        MutationOp::Move {
            id: 1,
            parent: 5,
            index: 0,
        },
    ])
    .unwrap();
    assert_eq!(children(&ucr, 0), [5, 2]);
    assert_eq!(children(&ucr, 5), [1]);
    assert_eq!(children(&ucr, 1), [3, 4]);
    assert_consistent(&ucr);
}

#[test]
fn move_rejects_invalid_targets() {
    assert_rejected(
        &[MutationOp::Move {
            id: 0,
            parent: 1,
            index: 0,
        }],
        0,
        MutationErrorReason::TargetsRoot,
    );
    assert_rejected(
        &[MutationOp::Move {
            id: 1,
            parent: 1,
            index: 0,
        }],
        0,
        MutationErrorReason::WouldCycle { id: 1, parent: 1 },
    );
    assert_rejected(
        &[
            insert(5, container(&[]), 1, 0),
            MutationOp::Move {
                id: 1,
                parent: 5,
                index: 0,
            },
        ],
        1,
        MutationErrorReason::WouldCycle { id: 1, parent: 5 },
    );
    assert_rejected(
        &[MutationOp::Move {
            id: 3,
            parent: 2,
            index: 0,
        }],
        0,
        MutationErrorReason::NotContainer { id: 2 },
    );
    assert_rejected(
        &[MutationOp::Move {
            id: 9,
            parent: 0,
            index: 0,
        }],
        0,
        MutationErrorReason::Unreachable { id: 9 },
    );
}

#[test]
fn update_replaces_given_fields_only() {
    let mut ucr = ucr();
    let position = Point::new(10.0, -20.0).unwrap();
    let applied = ucr
        .apply(&[
            MutationOp::Update {
                id: 2,
                position: Some(position),
                size: None,
                kind: None,
            },
            update_kind(3, text("changed")),
        ])
        .unwrap();
    let elements = &ucr.data().elements;
    assert_eq!(elements[&2].position, position);
    assert_eq!(elements[&2].size, Size::new(100.0, 100.0).unwrap());
    assert_eq!(elements[&2].kind, text("2"));
    assert_eq!(elements[&3].kind, text("changed"));
    assert_eq!(
        applied.updated().iter().copied().collect::<Vec<_>>(),
        [2, 3]
    );
    assert_consistent(&ucr);
}

#[test]
fn update_moves_and_resizes_container_and_root() {
    let mut ucr = ucr();
    let size = Size::new(640.0, 480.0).unwrap();
    ucr.apply(&[MutationOp::Update {
        id: 0,
        position: None,
        size: Some(size),
        kind: None,
    }])
    .unwrap();
    assert_eq!(ucr.data().elements[&0].size, size);
    assert_eq!(children(&ucr, 0), [1, 2]);
}

#[test]
fn update_rejects_invalid_targets() {
    assert_rejected(
        &[update_kind(2, button("2"))],
        0,
        MutationErrorReason::KindMismatch { id: 2 },
    );
    assert_rejected(
        &[update_kind(1, container(&[]))],
        0,
        MutationErrorReason::ContainerUpdate { id: 1 },
    );
    assert_rejected(
        &[update_kind(8, text("8"))],
        0,
        MutationErrorReason::ElementNotFound { id: 8 },
    );
    assert_rejected(
        &[update_kind(9, text("9"))],
        0,
        MutationErrorReason::Unreachable { id: 9 },
    );
}

#[test]
fn failure_discards_earlier_operations() {
    assert_rejected(
        &[
            insert(5, text("5"), 0, 0),
            MutationOp::Remove { id: 1 },
            update_kind(2, text("changed")),
            MutationOp::Remove { id: 4 },
        ],
        3,
        MutationErrorReason::ElementNotFound { id: 4 },
    );
}
