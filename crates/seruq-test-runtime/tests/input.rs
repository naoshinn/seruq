//! Button presses, driven without a window.

use seruq_ffr::{Functions, call};
use seruq_test_runtime::{Input, Runtime};
use seruq_ucr::{
    Element, ElementId, ElementKind, Event, EventKind, FfrCall, FontSize, Metadata, MutationOp,
    Point, Size, Timing, Ucr, UcrDraft, Value, ValueMap,
};

fn element(x: f32, y: f32, width: f32, height: f32, kind: ElementKind) -> Element {
    Element::new(
        Point::new(x, y).unwrap(),
        Size::new(width, height).unwrap(),
        kind,
    )
}

fn container(children: &[ElementId]) -> ElementKind {
    ElementKind::Container {
        children: children.to_vec(),
    }
}

// Empty labels and texts need no font, so these tests run without font assets.
fn button() -> ElementKind {
    ElementKind::Button {
        label: String::new(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

fn text() -> ElementKind {
    ElementKind::Text {
        text: String::new(),
        font_size: FontSize::new(16.0).unwrap(),
        font: None,
    }
}

fn runtime(elements: impl IntoIterator<Item = (ElementId, Element)>) -> Runtime {
    let (document, _) = UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: 0,
        elements: elements.into_iter().collect(),
        state: ValueMap::new(),
    }
    .build()
    .unwrap();
    Runtime::new(document, ".")
}

const A: ElementId = 1;
const B: ElementId = 2;
const LABEL: ElementId = 3;
const COVERED: ElementId = 4;
const CLIPPED: ElementId = 7;

/// 0 (0,0 400x200): [A, B, LABEL, COVERED, 5, 6]
///   A (10,10 100x40) Button
///   B (120,10 100x40) Button
///   LABEL (230,10 100x40) Text
///   COVERED (10,60 100x40) Button, with the left half under Text 8
///   5 (10,60 50x40) Container: [8 Text filling it]
///   6 (10,110 50x40) Container: [CLIPPED (0,0 100x40) Button]  -- right half clipped
fn scene() -> Runtime {
    runtime([
        (
            0,
            element(
                0.0,
                0.0,
                400.0,
                200.0,
                container(&[A, B, LABEL, COVERED, 5, 6]),
            ),
        ),
        (A, element(10.0, 10.0, 100.0, 40.0, button())),
        (B, element(120.0, 10.0, 100.0, 40.0, button())),
        (LABEL, element(230.0, 10.0, 100.0, 40.0, text())),
        (COVERED, element(10.0, 60.0, 100.0, 40.0, button())),
        (5, element(10.0, 60.0, 50.0, 40.0, container(&[8]))),
        (8, element(0.0, 0.0, 50.0, 40.0, text())),
        (6, element(10.0, 110.0, 50.0, 40.0, container(&[CLIPPED]))),
        (CLIPPED, element(0.0, 0.0, 100.0, 40.0, button())),
    ])
}

fn events(runtime: &mut Runtime, input: Input) -> Vec<Event> {
    runtime.handle_input(input);
    std::iter::from_fn(|| runtime.next_event()).collect()
}

fn press(runtime: &mut Runtime, x: f64, y: f64) -> Vec<Event> {
    events(runtime, Input::PointerPressed { x, y })
}

fn release(runtime: &mut Runtime, x: f64, y: f64) -> Vec<Event> {
    events(runtime, Input::PointerReleased { x, y })
}

fn moved(runtime: &mut Runtime, x: f64, y: f64) {
    assert!(events(runtime, Input::PointerMoved { x, y }).is_empty());
}

fn pushed(target: ElementId) -> Event {
    Event::new(target, EventKind::Pushed)
}

fn released(target: ElementId) -> Event {
    Event::new(target, EventKind::Released)
}

fn activated(target: ElementId) -> Event {
    Event::new(target, EventKind::Activated)
}

#[test]
fn click_pushes_releases_then_activates() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    assert_eq!(runtime.pressed(), Some(A));
    assert_eq!(
        release(&mut runtime, 100.0, 40.0),
        [released(A), activated(A)]
    );
    assert_eq!(runtime.pressed(), None);

    // The state is clean for the next click.
    assert_eq!(press(&mut runtime, 130.0, 20.0), [pushed(B)]);
    assert_eq!(
        release(&mut runtime, 130.0, 20.0),
        [released(B), activated(B)]
    );
}

#[test]
fn releasing_outside_cancels_activation() {
    let mut runtime = scene();
    // Over nothing, over another Button, and outside the viewport's content.
    for (x, y) in [(115.0, 20.0), (130.0, 20.0), (-5.0, 500.0)] {
        assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
        assert_eq!(release(&mut runtime, x, y), [released(A)]);
        assert_eq!(runtime.pressed(), None);
    }
}

#[test]
fn returning_before_release_activates() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    moved(&mut runtime, 130.0, 20.0);
    // The other Button is neither pressed nor shown as pressed.
    assert_eq!(runtime.pressed(), None);
    moved(&mut runtime, 30.0, 30.0);
    assert_eq!(runtime.pressed(), Some(A));
    assert_eq!(
        release(&mut runtime, 30.0, 30.0),
        [released(A), activated(A)]
    );
}

#[test]
fn leaving_the_viewport_keeps_the_press_until_release() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    assert!(events(&mut runtime, Input::PointerLeft).is_empty());
    assert_eq!(runtime.pressed(), None);
    moved(&mut runtime, 20.0, 20.0);
    assert_eq!(runtime.pressed(), Some(A));
    assert_eq!(
        release(&mut runtime, 20.0, 20.0),
        [released(A), activated(A)]
    );
}

#[test]
fn cancelled_press_only_releases() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    assert_eq!(events(&mut runtime, Input::PointerCancelled), [released(A)]);
    assert_eq!(runtime.pressed(), None);
    // The physical release that follows belongs to no press.
    assert!(release(&mut runtime, 20.0, 20.0).is_empty());
    assert!(events(&mut runtime, Input::PointerCancelled).is_empty());
}

#[test]
fn only_buttons_respond() {
    let mut runtime = scene();
    // Empty space, a Text, and a Container's empty area.
    for (x, y) in [(350.0, 150.0), (240.0, 20.0), (5.0, 5.0)] {
        assert!(press(&mut runtime, x, y).is_empty());
        assert_eq!(runtime.pressed(), None);
        // Releasing over a Button does not activate it without a press on it.
        assert!(release(&mut runtime, 20.0, 20.0).is_empty());
    }
}

#[test]
fn elements_in_front_block_the_button() {
    let mut runtime = scene();
    // The left half of COVERED is under a Text.
    assert!(press(&mut runtime, 30.0, 80.0).is_empty());
    assert!(release(&mut runtime, 30.0, 80.0).is_empty());

    assert_eq!(press(&mut runtime, 80.0, 80.0), [pushed(COVERED)]);
    // Releasing over the covered half is releasing over another element.
    assert_eq!(release(&mut runtime, 30.0, 80.0), [released(COVERED)]);
}

#[test]
fn clipped_regions_do_not_respond() {
    let mut runtime = scene();
    assert!(press(&mut runtime, 80.0, 130.0).is_empty());
    assert_eq!(press(&mut runtime, 30.0, 130.0), [pushed(CLIPPED)]);
    assert_eq!(release(&mut runtime, 80.0, 130.0), [released(CLIPPED)]);
}

#[test]
fn second_press_is_ignored_while_one_is_in_progress() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    assert!(press(&mut runtime, 130.0, 20.0).is_empty());
    assert_eq!(
        release(&mut runtime, 20.0, 20.0),
        [released(A), activated(A)]
    );
}

#[test]
fn removing_the_pressed_button_drops_the_press() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    runtime.apply(&[MutationOp::Remove { id: A }]).unwrap();
    assert_eq!(runtime.pressed(), None);
    assert!(release(&mut runtime, 20.0, 20.0).is_empty());
}

#[test]
fn reusing_the_pressed_buttons_id_drops_the_press() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    runtime
        .apply(&[
            MutationOp::Remove { id: A },
            MutationOp::Insert {
                id: A,
                element: element(10.0, 10.0, 100.0, 40.0, button()),
                parent: 0,
                index: 0,
            },
        ])
        .unwrap();
    assert!(release(&mut runtime, 20.0, 20.0).is_empty());
}

#[test]
fn unrelated_and_failed_mutations_keep_the_press() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    runtime.apply(&[MutationOp::Remove { id: B }]).unwrap();
    assert!(runtime.apply(&[MutationOp::Remove { id: 0 }]).is_err());
    assert_eq!(runtime.pressed(), Some(A));
    assert_eq!(
        release(&mut runtime, 20.0, 20.0),
        [released(A), activated(A)]
    );
}

#[test]
fn moving_the_button_away_prevents_activation() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 20.0, 20.0), [pushed(A)]);
    runtime
        .apply(&[MutationOp::Update {
            id: A,
            position: Some(Point::new(10.0, 150.0).unwrap()),
            size: None,
            kind: None,
        }])
        .unwrap();
    // The pointer has not moved, but the Button is no longer under it.
    assert_eq!(runtime.pressed(), None);
    assert_eq!(release(&mut runtime, 20.0, 20.0), [released(A)]);
}

fn pixel(frame: &seruq_test_runtime::Frame, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * frame.width() + x) * 4) as usize;
    frame.rgba()[offset..offset + 4].try_into().unwrap()
}

#[test]
fn pressed_button_is_drawn_differently_only_while_under_the_pointer() {
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const BLACK: [u8; 4] = [0, 0, 0, 255];
    let mut runtime = scene();
    let interior =
        |runtime: &mut Runtime, x: u32| pixel(&runtime.render(400, 200, 1.0).unwrap(), x, 30);

    assert_eq!(interior(&mut runtime, 60), WHITE);
    press(&mut runtime, 20.0, 20.0);
    let pressed = interior(&mut runtime, 60);
    assert_ne!(pressed, WHITE);
    assert_ne!(pressed, BLACK);
    // The border and the other Button are unchanged.
    assert_eq!(interior(&mut runtime, 10), BLACK);
    assert_eq!(interior(&mut runtime, 170), WHITE);

    moved(&mut runtime, 300.0, 150.0);
    assert_eq!(interior(&mut runtime, 60), WHITE);
    moved(&mut runtime, 20.0, 20.0);
    assert_eq!(interior(&mut runtime, 60), pressed);
    release(&mut runtime, 20.0, 20.0);
    assert_eq!(interior(&mut runtime, 60), WHITE);
}

/// The loop an application runs: one event at a time, applying each mutation
/// before the next event is resolved.
fn dispatch(runtime: &mut Runtime, functions: &mut Functions, input: Input) {
    runtime.handle_input(input);
    while let Some(event) = runtime.next_event() {
        if let Some(mutation) = call(functions, runtime.document(), &event).unwrap() {
            runtime.apply(&mutation).unwrap();
        }
    }
}

fn click(runtime: &mut Runtime, functions: &mut Functions, x: f64, y: f64) {
    dispatch(runtime, functions, Input::PointerPressed { x, y });
    dispatch(runtime, functions, Input::PointerReleased { x, y });
}

fn with_action(mut element: Element, timing: Timing, function_name: &str) -> Element {
    element.actions.insert(timing, FfrCall::new(function_name));
    element
}

#[test]
fn clicks_reach_functions_through_actions() {
    let mut runtime = runtime([
        (0, element(0.0, 0.0, 400.0, 200.0, container(&[A, B]))),
        (
            A,
            with_action(
                element(10.0, 10.0, 100.0, 40.0, button()),
                Timing::Activated,
                "count",
            ),
        ),
        // A Button without actions produces events that call nothing.
        (B, element(120.0, 10.0, 100.0, 40.0, button())),
    ]);
    let mut functions = Functions::new();
    functions.register("count", |_: &Event, document: &Ucr| {
        let count = document
            .data()
            .state
            .get("count")
            .and_then(Value::as_number)
            .unwrap_or(0.0);
        Ok(vec![MutationOp::SetState {
            key: "count".into(),
            value: Value::from(count as u32 + 1),
        }])
    });

    click(&mut runtime, &mut functions, 20.0, 20.0);
    click(&mut runtime, &mut functions, 130.0, 20.0);
    click(&mut runtime, &mut functions, 20.0, 20.0);
    // A press that is released elsewhere does not count.
    dispatch(
        &mut runtime,
        &mut functions,
        Input::PointerPressed { x: 20.0, y: 20.0 },
    );
    dispatch(
        &mut runtime,
        &mut functions,
        Input::PointerReleased { x: 300.0, y: 150.0 },
    );

    assert_eq!(runtime.document().data().state["count"], Value::from(2));
}

#[test]
fn function_that_removes_its_button_on_push_discards_the_rest_of_the_click() {
    let mut runtime = runtime([
        (0, element(0.0, 0.0, 400.0, 200.0, container(&[A]))),
        (
            A,
            with_action(
                with_action(
                    element(10.0, 10.0, 100.0, 40.0, button()),
                    Timing::Pushed,
                    "remove",
                ),
                Timing::Activated,
                "never",
            ),
        ),
    ]);
    let mut functions = Functions::new();
    functions
        .register("remove", |event: &Event, _: &Ucr| {
            Ok(vec![MutationOp::Remove { id: event.target }])
        })
        .register("never", |_: &Event, _: &Ucr| {
            panic!("a removed Button must not be activated")
        });

    click(&mut runtime, &mut functions, 20.0, 20.0);
    assert_eq!(runtime.layout().len(), 1);
}

#[test]
fn function_that_removes_its_button_on_release_discards_the_activation() {
    let mut runtime = runtime([
        (0, element(0.0, 0.0, 400.0, 200.0, container(&[A]))),
        (
            A,
            with_action(
                with_action(
                    element(10.0, 10.0, 100.0, 40.0, button()),
                    Timing::Released,
                    "remove",
                ),
                Timing::Activated,
                "never",
            ),
        ),
    ]);
    let mut functions = Functions::new();
    functions
        .register("remove", |event: &Event, _: &Ucr| {
            Ok(vec![MutationOp::Remove { id: event.target }])
        })
        .register("never", |_: &Event, _: &Ucr| {
            panic!("a removed Button must not be activated")
        });

    // Released and Activated come from one input; the second is resolved
    // against the document left by the first.
    click(&mut runtime, &mut functions, 20.0, 20.0);
    assert_eq!(runtime.layout().len(), 1);
}

#[test]
fn replacing_a_button_on_release_does_not_activate_its_replacement() {
    let mut runtime = runtime([
        (0, element(0.0, 0.0, 400.0, 200.0, container(&[A]))),
        (
            A,
            with_action(
                element(10.0, 10.0, 100.0, 40.0, button()),
                Timing::Released,
                "replace",
            ),
        ),
    ]);
    let mut functions = Functions::new();
    functions
        .register("replace", |event: &Event, _: &Ucr| {
            Ok(vec![
                MutationOp::Remove { id: event.target },
                MutationOp::Insert {
                    id: event.target,
                    element: with_action(
                        element(10.0, 10.0, 100.0, 40.0, button()),
                        Timing::Activated,
                        "count",
                    ),
                    parent: 0,
                    index: 0,
                },
            ])
        })
        .register("count", |_: &Event, _: &Ucr| {
            Ok(vec![MutationOp::SetState {
                key: "count".into(),
                value: Value::from(1),
            }])
        });

    click(&mut runtime, &mut functions, 20.0, 20.0);
    assert!(!runtime.document().data().state.contains_key("count"));

    // Reusing the ID does not prevent a fresh click on the replacement.
    click(&mut runtime, &mut functions, 20.0, 20.0);
    assert_eq!(runtime.document().data().state["count"], Value::from(1));
}

#[test]
fn replacing_a_subtree_discards_queued_events_for_its_descendants() {
    let mut runtime = scene();
    assert_eq!(press(&mut runtime, 30.0, 130.0), [pushed(CLIPPED)]);
    runtime.handle_input(Input::PointerReleased { x: 30.0, y: 130.0 });
    assert_eq!(runtime.next_event(), Some(released(CLIPPED)));

    // Remove the ancestor, then reuse both its ID and the Button's ID in a
    // separate mutation. The old activation must never reach the host handler.
    runtime.apply(&[MutationOp::Remove { id: 6 }]).unwrap();
    runtime
        .apply(&[
            MutationOp::Insert {
                id: 6,
                element: element(10.0, 110.0, 50.0, 40.0, container(&[])),
                parent: 0,
                index: 5,
            },
            MutationOp::Insert {
                id: CLIPPED,
                element: element(0.0, 0.0, 100.0, 40.0, button()),
                parent: 6,
                index: 0,
            },
        ])
        .unwrap();
    assert_eq!(runtime.next_event(), None);
    assert_eq!(press(&mut runtime, 30.0, 130.0), [pushed(CLIPPED)]);
    assert_eq!(
        release(&mut runtime, 30.0, 130.0),
        [released(CLIPPED), activated(CLIPPED)]
    );
}

#[test]
fn updates_and_failed_mutations_preserve_queued_activation() {
    let mut runtime = scene();
    press(&mut runtime, 20.0, 20.0);
    runtime.handle_input(Input::PointerReleased { x: 20.0, y: 20.0 });
    assert_eq!(runtime.next_event(), Some(released(A)));

    // Neither an unrelated removal nor an update replaces the event's target.
    runtime
        .apply(&[
            MutationOp::Remove { id: B },
            MutationOp::Update {
                id: A,
                position: Some(Point::new(10.0, 150.0).unwrap()),
                size: None,
                kind: None,
            },
        ])
        .unwrap();
    // The attempted removal is rolled back, so its queued event must survive.
    assert!(
        runtime
            .apply(&[MutationOp::Remove { id: A }, MutationOp::Remove { id: 0 }])
            .is_err()
    );
    assert_eq!(runtime.next_event(), Some(activated(A)));
    assert_eq!(runtime.next_event(), None);
}
