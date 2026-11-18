use std::{cell::RefCell, rc::Rc};

use seruq_ffr::{CallErrorReason, FunctionHost, Functions, Unresolved, call, unresolved};
use seruq_ucr::{
    Element, ElementId, ElementKind, Event, EventKind, FfrCall, FontSize, Metadata, Mutation,
    MutationOp, Point, Size, Timing, Ucr, UcrDraft, Value, ValueMap,
};

fn element(kind: ElementKind, actions: &[(Timing, &str)]) -> Element {
    let mut element = Element::new(
        Point::new(0.0, 0.0).unwrap(),
        Size::new(100.0, 40.0).unwrap(),
        kind,
    );
    for &(timing, function_name) in actions {
        element.actions.insert(timing, FfrCall::new(function_name));
    }
    element
}

fn text(text: &str) -> ElementKind {
    ElementKind::Text {
        text: text.into(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

fn button() -> ElementKind {
    ElementKind::Button {
        label: "+1".into(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

const ROOT: ElementId = 0;
const LABEL: ElementId = 1;
const INCREMENT: ElementId = 2;
const PLAIN: ElementId = 3;
const UNUSED: ElementId = 9;

/// A counter: `state.count`, a Text showing it, and a Button that increments it.
fn counter() -> Ucr {
    UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: ROOT,
        elements: [
            (
                ROOT,
                element(
                    ElementKind::Container {
                        children: vec![LABEL, INCREMENT, PLAIN],
                    },
                    &[],
                ),
            ),
            (LABEL, element(text("0"), &[])),
            (
                INCREMENT,
                element(
                    button(),
                    &[
                        (Timing::Activated, "counter.increment"),
                        (Timing::Pushed, "counter.missing"),
                    ],
                ),
            ),
            (PLAIN, element(button(), &[])),
            (
                UNUSED,
                element(button(), &[(Timing::Activated, "counter.unused")]),
            ),
        ]
        .into(),
        state: ValueMap::from([("count".to_owned(), Value::from(0))]),
    }
    .build()
    .unwrap()
    .0
}

fn increment(_event: &Event, document: &Ucr) -> Result<Mutation, seruq_ffr::FunctionError> {
    let count = document
        .data()
        .state
        .get("count")
        .and_then(Value::as_number)
        .ok_or("state.count is not a number")?
        + 1.0;
    Ok(vec![
        MutationOp::SetState {
            key: "count".into(),
            value: Value::Number(seruq_ucr::Number::new(count)?),
        },
        MutationOp::Update {
            id: LABEL,
            position: None,
            size: None,
            kind: Some(text(&count.to_string())),
        },
    ])
}

fn functions() -> Functions {
    let mut functions = Functions::new();
    functions.register("counter.increment", increment);
    functions
}

fn activated(target: ElementId) -> Event {
    Event::new(target, EventKind::Activated)
}

#[test]
fn function_reads_state_and_returns_mutation() {
    let mut document = counter();
    let mut functions = functions();

    for expected in ["1", "2", "3"] {
        let mutation = call(&mut functions, &document, &activated(INCREMENT))
            .unwrap()
            .unwrap();
        // The call itself changes nothing.
        assert_ne!(document.data().elements[&LABEL].kind, text(expected));
        let applied = document.apply(&mutation).unwrap();
        assert_eq!(document.data().elements[&LABEL].kind, text(expected));
        assert!(applied.state().contains("count"));
        assert!(applied.updated().contains(&LABEL));
    }
    assert_eq!(document.data().state["count"], Value::from(3));
}

#[test]
fn events_without_an_action_call_nothing() {
    let document = counter();
    let mut functions = Functions::new();
    functions.register("counter.increment", |_: &Event, _: &Ucr| {
        panic!("no function should be called")
    });

    // No action for this timing, no actions at all, and a Container.
    for event in [
        Event::new(INCREMENT, EventKind::Released),
        activated(PLAIN),
        activated(ROOT),
        Event::new(
            PLAIN,
            EventKind::ValueChanged {
                value: "text".into(),
            },
        ),
    ] {
        assert!(call(&mut functions, &document, &event).unwrap().is_none());
    }
}

#[test]
fn events_for_missing_or_unreachable_targets_are_discarded() {
    let mut document = counter();
    let mut functions = functions();
    functions.register("counter.unused", |_: &Event, _: &Ucr| {
        panic!("an unreachable element's action must not run")
    });

    assert!(
        call(&mut functions, &document, &activated(UNUSED))
            .unwrap()
            .is_none()
    );
    assert!(
        call(&mut functions, &document, &activated(404))
            .unwrap()
            .is_none()
    );

    // An event that was produced before its target was removed.
    let event = activated(INCREMENT);
    document
        .apply(&[MutationOp::Remove { id: INCREMENT }])
        .unwrap();
    assert!(call(&mut functions, &document, &event).unwrap().is_none());
}

#[test]
fn unknown_function_is_reported_with_its_action() {
    let document = counter();
    let error = call(
        &mut functions(),
        &document,
        &Event::new(INCREMENT, EventKind::Pushed),
    )
    .unwrap_err();
    assert_eq!(error.element, INCREMENT);
    assert_eq!(error.timing, Timing::Pushed);
    assert_eq!(error.function_name, "counter.missing");
    assert!(matches!(error.reason, CallErrorReason::UnknownFunction));
    assert_eq!(
        error.to_string(),
        r#"unknown function "counter.missing" for Pushed on element 2"#
    );
}

#[test]
fn function_failure_is_reported_and_changes_nothing() {
    let mut document = counter();
    document
        .apply(&[MutationOp::SetState {
            key: "count".into(),
            value: Value::from("three"),
        }])
        .unwrap();
    let before = document.clone();

    let error = call(&mut functions(), &document, &activated(INCREMENT)).unwrap_err();
    assert_eq!(error.function_name, "counter.increment");
    assert!(matches!(error.reason, CallErrorReason::Function(_)));
    assert_eq!(
        error.to_string(),
        r#"function "counter.increment" failed for Activated on element 2: state.count is not a number"#
    );
    assert_eq!(document, before);
}

#[test]
fn rejected_mutation_leaves_document_unchanged() {
    let mut document = counter();
    let mut functions = Functions::new();
    functions.register("counter.increment", |_: &Event, _: &Ucr| {
        Ok(vec![
            MutationOp::SetState {
                key: "count".into(),
                value: Value::from(1),
            },
            MutationOp::Remove { id: ROOT },
        ])
    });
    let before = document.clone();

    let mutation = call(&mut functions, &document, &activated(INCREMENT))
        .unwrap()
        .unwrap();
    assert_eq!(document.apply(&mutation).unwrap_err().index, 1);
    assert_eq!(document, before);
}

#[test]
fn function_receives_the_event_and_can_keep_its_own_resources() {
    let document = counter();
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut functions = Functions::new();
    let sink = log.clone();
    functions.register("counter.increment", move |event: &Event, _: &Ucr| {
        sink.borrow_mut().push(event.clone());
        Ok(Mutation::new())
    });

    let event = activated(INCREMENT);
    for _ in 0..2 {
        let mutation = call(&mut functions, &document, &event).unwrap().unwrap();
        assert!(mutation.is_empty());
    }
    assert_eq!(*log.borrow(), [event.clone(), event]);
}

#[test]
fn registering_a_name_again_replaces_the_function() {
    let document = counter();
    let mut functions = Functions::new();
    functions
        .register("counter.increment", |_: &Event, _: &Ucr| {
            Err("replaced".into())
        })
        .register(
            "counter.increment",
            |_: &Event, _: &Ucr| Ok(Mutation::new()),
        );
    assert!(
        call(&mut functions, &document, &activated(INCREMENT))
            .unwrap()
            .is_some()
    );
}

#[test]
fn unresolved_lists_reachable_actions_without_functions() {
    let document = counter();
    assert_eq!(
        unresolved(&Functions::new(), &document),
        [
            Unresolved {
                element: INCREMENT,
                timing: Timing::Pushed,
                function_name: "counter.missing".into(),
            },
            Unresolved {
                element: INCREMENT,
                timing: Timing::Activated,
                function_name: "counter.increment".into(),
            },
        ]
    );

    let mut functions = functions();
    assert_eq!(unresolved(&functions, &document).len(), 1);
    functions.register("counter.missing", |_: &Event, _: &Ucr| Ok(Mutation::new()));
    assert!(unresolved(&functions, &document).is_empty());
}

/// A host for functions that are not Rust closures only needs these two methods.
struct ConstantHost;

impl FunctionHost for ConstantHost {
    fn contains(&self, function_name: &str) -> bool {
        function_name == "counter.increment"
    }

    fn invoke(
        &mut self,
        function_name: &str,
        event: &Event,
        _document: &Ucr,
    ) -> Option<Result<Mutation, seruq_ffr::FunctionError>> {
        self.contains(function_name).then(|| {
            Ok(vec![MutationOp::SetData {
                id: event.target,
                key: "called".into(),
                value: Value::from(true),
            }])
        })
    }
}

#[test]
fn custom_hosts_run_through_the_same_call() {
    let mut document = counter();
    let mutation = call(&mut ConstantHost, &document, &activated(INCREMENT))
        .unwrap()
        .unwrap();
    document.apply(&mutation).unwrap();
    assert_eq!(
        document.data().elements[&INCREMENT].data["called"],
        Value::from(true)
    );
    assert!(matches!(
        call(
            &mut ConstantHost,
            &document,
            &Event::new(INCREMENT, EventKind::Pushed)
        )
        .unwrap_err()
        .reason,
        CallErrorReason::UnknownFunction
    ));
}
