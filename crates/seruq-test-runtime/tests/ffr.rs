//! A click travels from a logical point to a mutation of the runtime's document.

use seruq_ffr::{Functions, call};
use seruq_test_runtime::Runtime;
use seruq_ucr::{
    Element, ElementId, ElementKind, Event, EventKind, FfrCall, FontSize, Metadata, MutationOp,
    Point, Size, Timing, Ucr, UcrDraft, Value, ValueMap,
};

const ROOT: ElementId = 0;
const ADD: ElementId = 1;
const FIRST_ROW: ElementId = 100;

fn button(x: f32, y: f32, function_name: &str) -> Element {
    let mut element = Element::new(
        Point::new(x, y).unwrap(),
        Size::new(100.0, 30.0).unwrap(),
        ElementKind::Button {
            label: String::new(),
            font_size: FontSize::new(16.0).unwrap(),
            font: None,
            color: None,
            background: None,
        },
    );
    element
        .actions
        .insert(Timing::Activated, FfrCall::new(function_name));
    element
}

/// An "add" Button at the top. Each activation appends a row Button below the
/// previous rows; activating a row removes it.
fn runtime() -> Runtime {
    let root = Element::new(
        Point::new(0.0, 0.0).unwrap(),
        Size::new(100.0, 300.0).unwrap(),
        ElementKind::Container {
            children: vec![ADD],
            background: None,
        },
    );
    let (document, _) = UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: ROOT,
        elements: [(ROOT, root), (ADD, button(0.0, 0.0, "rows.add"))].into(),
        state: ValueMap::from([("next_row".to_owned(), Value::from(0))]),
    }
    .build()
    .unwrap();
    Runtime::new(document, ".")
}

fn functions() -> Functions {
    let mut functions = Functions::new();
    functions
        .register("rows.add", |_: &Event, document: &Ucr| {
            let row = document.data().state["next_row"]
                .as_number()
                .ok_or("state.next_row is not a number")?;
            let ElementKind::Container { children, .. } = &document.data().elements[&ROOT].kind
            else {
                return Err("the root is not a Container".into());
            };
            let mut element = button(0.0, 40.0 * children.len() as f32, "rows.remove");
            element.data.insert("row".into(), Value::from(row as u32));
            Ok(vec![
                MutationOp::Insert {
                    id: FIRST_ROW + row as ElementId,
                    element,
                    parent: ROOT,
                    index: children.len(),
                },
                MutationOp::SetState {
                    key: "next_row".into(),
                    value: Value::from(row as u32 + 1),
                },
            ])
        })
        .register("rows.remove", |event: &Event, _: &Ucr| {
            Ok(vec![MutationOp::Remove { id: event.target }])
        });
    functions
}

/// What a runtime's input handling will do for a completed click.
fn click(runtime: &mut Runtime, functions: &mut Functions, x: f64, y: f64) -> Option<ElementId> {
    let target = runtime.hit_test(x, y)?;
    let event = Event::new(target, EventKind::Activated);
    if let Some(mutation) = call(functions, runtime.document(), &event).unwrap() {
        runtime.apply(&mutation).unwrap();
    }
    Some(target)
}

#[test]
fn clicks_run_functions_that_change_what_later_clicks_hit() {
    let mut runtime = runtime();
    let mut functions = functions();
    assert!(seruq_ffr::unresolved(&functions, runtime.document()).is_empty());

    // Nothing is below the "add" Button yet.
    assert_eq!(click(&mut runtime, &mut functions, 50.0, 50.0), None);

    assert_eq!(click(&mut runtime, &mut functions, 50.0, 10.0), Some(ADD));
    assert_eq!(click(&mut runtime, &mut functions, 50.0, 10.0), Some(ADD));
    assert_eq!(runtime.layout().len(), 4);
    assert_eq!(runtime.document().data().state["next_row"], Value::from(2));
    assert_eq!(
        runtime.document().data().elements[&(FIRST_ROW + 1)].data["row"],
        Value::from(1)
    );

    // The first row now receives the click that hit nothing before, and removes itself.
    assert_eq!(
        click(&mut runtime, &mut functions, 50.0, 50.0),
        Some(FIRST_ROW)
    );
    assert_eq!(click(&mut runtime, &mut functions, 50.0, 50.0), None);
    assert_eq!(
        click(&mut runtime, &mut functions, 50.0, 90.0),
        Some(FIRST_ROW + 1)
    );
    assert_eq!(runtime.layout().len(), 2);

    // The removed rows' IDs are not reused, because the counter lives in the state.
    assert_eq!(click(&mut runtime, &mut functions, 50.0, 10.0), Some(ADD));
    assert!(
        runtime
            .document()
            .data()
            .elements
            .contains_key(&(FIRST_ROW + 2))
    );
}
