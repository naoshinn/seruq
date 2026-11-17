//! A clickable counter: JSON document, pointer input, FFR, and mutations together.
//!
//! The document holds the count in `state.count` and shows it in a Text. Each
//! Button names a function in its `actions`; the "+1" and "+10" Buttons share
//! one function and differ only in their `data.step`.

use std::{error::Error, io, path::PathBuf};

use seruq_ffr::{FunctionError, Functions, call, unresolved};
use seruq_test_runtime::{Input, Runtime, WindowOptions};
use seruq_ucr::{
    ElementId, ElementKind, Event, FontSize, Mutation, MutationOp, Number, Ucr, Value,
};

const LABEL: ElementId = 1;

/// The operations that store a count and show it.
fn show(count: f64) -> Result<Mutation, FunctionError> {
    Ok(vec![
        MutationOp::SetState {
            key: "count".into(),
            value: Number::new(count)?.into(),
        },
        MutationOp::Update {
            id: LABEL,
            position: None,
            size: None,
            kind: Some(ElementKind::Text {
                text: format!("Count: {count}"),
                font_size: FontSize::new(26.0)?,
                font: None,
            }),
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
            show(count + step)
        })
        .register("counter.reset", |_: &Event, _: &Ucr| show(0.0));
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
                for event in runtime.handle_input(input) {
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
        runtime.run_with(options, move |runtime, event| {
            handle(runtime, &mut functions, event)
        })?;
    }
    Ok(())
}
