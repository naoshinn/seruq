//! A clickable counter: JSON document, pointer input, FFR, and mutations together.
//!
//! The document holds the count in `state.count` and shows it in a Text. Each
//! Button names a function in its `actions`; the "+1" and "+10" Buttons share
//! one function and differ only in their `data.step`.

use std::{error::Error, io, path::PathBuf};

use seruq_ffr::{FunctionError, Functions, call, unresolved};
use seruq_test_runtime::{Input, Runtime, RuntimeEvent, Viewport, WindowOptions};
use seruq_ucr::{
    ElementId, ElementKind, Event, Mutation, MutationOp, Number, Point, Size, Ucr, Value,
};

const LABEL: ElementId = 1;

/// The operations that store a count and show it.
fn show(count: f64, document: &Ucr) -> Result<Mutation, FunctionError> {
    let mut kind = document
        .data()
        .elements
        .get(&LABEL)
        .ok_or("missing counter label")?
        .kind
        .clone();
    let ElementKind::Text { text, .. } = &mut kind else {
        return Err("counter label is not Text".into());
    };
    *text = format!("Count: {count}");
    Ok(vec![
        MutationOp::SetState {
            key: "count".into(),
            value: Number::new(count)?.into(),
        },
        MutationOp::Update {
            id: LABEL,
            position: None,
            size: None,
            kind: Some(kind),
        },
    ])
}

fn functions() -> Functions {
    let mut functions = Functions::new();
    functions
        .register("counter.add", |event: &Event, document: &Ucr| {
            let data = document.data();
            let count = data
                .state
                .get("count")
                .and_then(Value::as_number)
                .ok_or("state.count is not a number")?;
            let step = data.elements[&event.target]
                .data
                .get("step")
                .and_then(Value::as_number)
                .ok_or("the target's data.step is not a number")?;
            show(count + step, document)
        })
        .register("counter.reset", |_: &Event, document: &Ucr| {
            show(0.0, document)
        });
    functions
}

/// Resolve one event and apply its mutation. Failures leave the document unchanged.
fn handle(runtime: &mut Runtime, functions: &mut Functions, event: Event) {
    match call(functions, runtime.document(), &event) {
        Ok(Some(mutation)) => {
            if let Err(error) = runtime.apply(&mutation) {
                eprintln!("{error}");
            }
        }
        Ok(None) => {}
        Err(error) => eprintln!("{error}"),
    }
}

/// Layout belongs to the application: wrap controls when the viewport is narrow.
fn layout(runtime: &mut Runtime, viewport: Viewport) -> Result<(), Box<dyn Error>> {
    let width = viewport.width.min(f64::from(f32::MAX)) as f32;
    let height = viewport.height.min(f64::from(f32::MAX)) as f32;
    let columns = (((width - 48.0) / 112.0).floor() as usize).clamp(1, 3);
    let mut mutation = vec![
        MutationOp::Update {
            id: runtime.document().data().root,
            position: None,
            size: Some(Size::new(width, height)?),
            kind: None,
        },
        MutationOp::Update {
            id: LABEL,
            position: None,
            size: Some(Size::new((width - 48.0).max(0.0), 40.0)?),
            kind: None,
        },
    ];
    for (index, id) in [2, 3, 4].into_iter().enumerate() {
        mutation.push(MutationOp::Update {
            id,
            position: Some(Point::new(
                24.0 + (index % columns) as f32 * 112.0,
                80.0 + (index / columns) as f32 * 60.0,
            )?),
            size: None,
            kind: None,
        });
    }
    runtime.apply(&mutation)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "usage: counter [output.png]").into(),
        );
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/counter.json");
    let draft = seruq_ucr_json::from_str(&std::fs::read_to_string(&path)?)?;
    let (document, _) = draft.build().map_err(|report| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("UCR validation failed: {:?}", report.errors()),
        )
    })?;

    let mut functions = functions();
    let missing = unresolved(&functions, &document);
    if !missing.is_empty() {
        return Err(format!("actions without functions: {missing:?}").into());
    }

    let mut runtime = Runtime::new(document, path.parent().unwrap_or(&path));
    let options = WindowOptions {
        title: "Seruq Counter".into(),
        ..WindowOptions::default()
    };

    if let Some(output) = output {
        if let Some(RuntimeEvent::ViewportResized(viewport)) =
            runtime.resize(options.width, options.height, 1.0)?
        {
            layout(&mut runtime, viewport)?;
        }
        // Without a window: click "+10" twice and "+1" once, then hold "Reset".
        for (x, y, release) in [
            (180.0, 100.0, true),
            (180.0, 100.0, true),
            (70.0, 100.0, true),
            (290.0, 100.0, false),
        ] {
            let mut inputs = vec![Input::PointerPressed { x, y }];
            if release {
                inputs.push(Input::PointerReleased { x, y });
            }
            for input in inputs {
                runtime.handle_input(input);
                while let Some(event) = runtime.next_event() {
                    handle(&mut runtime, &mut functions, event);
                }
            }
        }
        runtime
            .render(options.width, options.height, 1.0)?
            .save_png(&output)?;
        println!(
            "count = {:?}, saved {}",
            runtime.document().data().state.get("count"),
            output.display()
        );
    } else {
        runtime.run_with_events(options, move |runtime, event| match event {
            RuntimeEvent::Element(event) => handle(runtime, &mut functions, event),
            RuntimeEvent::ViewportResized(viewport) => {
                if let Err(error) = layout(runtime, viewport) {
                    eprintln!("{error}");
                }
            }
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_layout_wraps_buttons_and_keeps_actions_working() {
        let draft = seruq_ucr_json::from_str(include_str!("counter.json")).unwrap();
        let (document, _) = draft.build().unwrap();
        let mut runtime = Runtime::new(document, ".");
        let original_color = match &runtime.document().data().elements[&LABEL].kind {
            ElementKind::Text { color, .. } => *color,
            _ => panic!("missing Text label"),
        };
        let mut functions = functions();
        for (width, height, x, y, expected) in [
            (240, 300, 30.0, 150.0, 10.0),
            (640, 480, 150.0, 100.0, 20.0),
        ] {
            let Some(RuntimeEvent::ViewportResized(viewport)) =
                runtime.resize(width, height, 1.0).unwrap()
            else {
                panic!("missing viewport event")
            };
            layout(&mut runtime, viewport).unwrap();
            assert_eq!(runtime.hit_test(x, y), Some(3));
            for input in [
                Input::PointerPressed { x, y },
                Input::PointerReleased { x, y },
            ] {
                runtime.handle_input(input);
                while let Some(event) = runtime.next_event() {
                    handle(&mut runtime, &mut functions, event);
                }
            }
            assert_eq!(
                runtime.document().data().state["count"].as_number(),
                Some(expected)
            );
            let ElementKind::Text { color, .. } = &runtime.document().data().elements[&LABEL].kind
            else {
                panic!("missing Text label");
            };
            assert_eq!(*color, original_color);
        }
    }
}
