//! A counter driven through FFR without a window.
//!
//! The document is the application's state: `state.count` holds the number and
//! a Text element shows it. Activating the Button calls `counter.increment`,
//! which reads the document and returns a mutation changing both.

use std::error::Error;

use seruq_ffr::{Functions, call, unresolved};
use seruq_ucr::{
    Element, ElementId, ElementKind, Event, EventKind, FfrCall, FontSize, Metadata, MutationOp,
    Number, Point, Size, Timing, Ucr, UcrDraft, Value, ValueMap,
};

const ROOT: ElementId = 0;
const LABEL: ElementId = 1;
const INCREMENT: ElementId = 2;

fn label(count: f64) -> Result<ElementKind, Box<dyn Error + Send + Sync>> {
    Ok(ElementKind::Text {
        text: format!("Count: {count}"),
        font_size: FontSize::new(24.0)?,
        font: None,
    })
}

fn document() -> Result<Ucr, Box<dyn Error + Send + Sync>> {
    let mut button = Element::new(
        Point::new(16.0, 64.0)?,
        Size::new(120.0, 40.0)?,
        ElementKind::Button {
            label: "+1".into(),
            font_size: FontSize::new(16.0)?,
            font: None,
        },
    );
    button
        .actions
        .insert(Timing::Activated, FfrCall::new("counter.increment"));
    // How much one activation adds is this Button's own data.
    button.data.insert("step".into(), Value::from(1));

    let draft = UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: ROOT,
        elements: [
            (
                ROOT,
                Element::new(
                    Point::new(0.0, 0.0)?,
                    Size::new(320.0, 120.0)?,
                    ElementKind::Container {
                        children: vec![LABEL, INCREMENT],
                    },
                ),
            ),
            (
                LABEL,
                Element::new(
                    Point::new(16.0, 16.0)?,
                    Size::new(288.0, 32.0)?,
                    label(0.0)?,
                ),
            ),
            (INCREMENT, button),
        ]
        .into(),
        state: ValueMap::from([("count".to_owned(), Value::from(0))]),
    };
    let (document, _) = draft
        .build()
        .map_err(|report| format!("invalid document: {:?}", report.errors()))?;
    Ok(document)
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut document = document()?;

    let mut functions = Functions::new();
    functions.register("counter.increment", |event: &Event, document: &Ucr| {
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
        let count = count + step;
        Ok(vec![
            MutationOp::SetState {
                key: "count".into(),
                value: Number::new(count)?.into(),
            },
            MutationOp::Update {
                id: LABEL,
                position: None,
                size: None,
                kind: Some(label(count)?),
            },
        ])
    });

    let missing = unresolved(&functions, &document);
    if !missing.is_empty() {
        return Err(format!("unresolved actions: {missing:?}").into());
    }

    // A runtime would produce this event from pointer or keyboard input.
    let event = Event::new(INCREMENT, EventKind::Activated);
    for _ in 0..3 {
        if let Some(mutation) = call(&mut functions, &document, &event)? {
            document.apply(&mutation)?;
        }
        if let ElementKind::Text { text, .. } = &document.data().elements[&LABEL].kind {
            println!("{text}");
        }
    }
    Ok(())
}
