use seruq_test_runtime::{Error, Input, Runtime, RuntimeEvent, Viewport};
use seruq_ucr::{
    Element, ElementKind, FontSize, Metadata, MutationOp, Point, Size, UcrDraft, ValueMap,
};

fn runtime() -> Runtime {
    let (document, _) = UcrDraft {
        metadata: Metadata::new("0.1.0".into(), None),
        root: 0,
        state: ValueMap::new(),
        elements: [
            (
                0,
                Element::new(
                    Point::new(0.0, 0.0).unwrap(),
                    Size::new(100.0, 100.0).unwrap(),
                    ElementKind::Container { children: vec![1] },
                ),
            ),
            (
                1,
                Element::new(
                    Point::new(10.0, 10.0).unwrap(),
                    Size::new(40.0, 40.0).unwrap(),
                    ElementKind::Button {
                        label: String::new(),
                        font_size: FontSize::new(16.0).unwrap(),
                        font: None,
                    },
                ),
            ),
        ]
        .into_iter()
        .collect(),
    }
    .build()
    .unwrap();
    Runtime::new(document, ".")
}

#[test]
fn initial_viewport_and_changes_are_reported_in_logical_units() {
    let mut runtime = runtime();
    assert_eq!(runtime.viewport(), None);
    let viewport = Viewport {
        width: 400.0,
        height: 300.0,
        scale_factor: 2.0,
    };
    assert_eq!(
        runtime.resize(800, 600, 2.0).unwrap(),
        Some(RuntimeEvent::ViewportResized(viewport))
    );
    assert_eq!(runtime.viewport(), Some(viewport));
    assert_eq!(runtime.resize(800, 600, 2.0).unwrap(), None);
    // A scale-only change is observable, even when logical dimensions stay equal.
    assert!(runtime.resize(400, 300, 1.0).unwrap().is_some());
    assert!(runtime.resize(500, 300, 1.0).unwrap().is_some());
}

#[test]
fn minimized_viewport_can_be_restored() {
    let mut runtime = runtime();
    runtime.resize(400, 300, 1.0).unwrap();
    assert!(runtime.resize(0, 0, 2.0).unwrap().is_some());
    assert_eq!(runtime.viewport().unwrap().width, 0.0);
    assert_eq!(runtime.resize(0, 0, 2.0).unwrap(), None);
    assert!(runtime.resize(400, 300, 1.0).unwrap().is_some());
}

#[test]
fn invalid_scales_preserve_previous_viewport() {
    let mut runtime = runtime();
    runtime.resize(400, 300, 1.0).unwrap();
    let previous = runtime.viewport();
    for scale in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(1),
    ] {
        assert!(matches!(
            runtime.resize(400, 300, scale),
            Err(Error::InvalidScaleFactor(_))
        ));
        assert_eq!(runtime.viewport(), previous);
    }
}

#[test]
fn resizing_does_not_implicitly_change_document_or_render_state() {
    let mut runtime = runtime();
    let before = runtime.document().clone();
    runtime.resize(800, 600, 2.0).unwrap();
    assert_eq!(runtime.document(), &before);
    let viewport = runtime.viewport();
    runtime.render(200, 100, 1.0).unwrap();
    assert_eq!(runtime.viewport(), viewport);
}

#[test]
fn application_layout_mutation_changes_clipping_and_input() {
    let mut runtime = runtime();
    runtime.handle_input(Input::PointerPressed { x: 20.0, y: 20.0 });
    let Some(RuntimeEvent::ViewportResized(viewport)) = runtime.resize(200, 100, 1.0).unwrap()
    else {
        panic!("missing initial viewport")
    };
    runtime
        .apply(&[
            MutationOp::Update {
                id: 0,
                position: None,
                size: Some(Size::new(viewport.width as f32, viewport.height as f32).unwrap()),
                kind: None,
            },
            MutationOp::Update {
                id: 1,
                position: Some(Point::new(150.0, 10.0).unwrap()),
                size: None,
                kind: None,
            },
        ])
        .unwrap();
    assert_eq!(runtime.hit_test(160.0, 20.0), Some(1));
    assert_eq!(runtime.hit_test(20.0, 20.0), None);
    assert_eq!(runtime.pressed(), None);
    // The original press survives a layout change and can finish on its target.
    assert_eq!(
        runtime
            .handle_input(Input::PointerReleased { x: 160.0, y: 20.0 })
            .len(),
        2
    );
    let frame = runtime.render(200, 100, 1.0).unwrap();
    let offset = ((20 * 200 + 150) * 4) as usize;
    assert_eq!(&frame.rgba()[offset..offset + 4], &[0, 0, 0, 255]);
}
