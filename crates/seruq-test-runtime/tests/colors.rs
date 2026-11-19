use seruq_test_runtime::{Frame, Input, Runtime};
use seruq_ucr::{Color, ElementKind, MutationOp};

fn runtime(json: &str) -> Runtime {
    let (document, _) = seruq_ucr_json::from_str(json).unwrap().build().unwrap();
    Runtime::new(
        document,
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets"),
    )
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * frame.width() + x) * 4) as usize;
    frame.rgba()[offset..offset + 4].try_into().unwrap()
}

#[test]
fn backgrounds_follow_paint_order_and_ancestor_clipping() {
    let mut runtime = runtime(
        r##"{
        "metadata":{"version":"0.1.0","background":"#123456"}, "root":"0",
        "elements":{
            "0":{"position":{"x":10,"y":10},"size":{"width":80,"height":60},"kind":{"type":"container","children":["1","2"],"background":"#abcdef"}},
            "1":{"position":{"x":-5,"y":5},"size":{"width":30,"height":20},"kind":{"type":"button","label":"","font_size":16,"background":"#ff0000"}},
            "2":{"position":{"x":10,"y":10},"size":{"width":100,"height":10},"kind":{"type":"container","children":[],"background":"#00ff00"}}
        }
    }"##,
    );
    for scale in [1, 2] {
        let frame = runtime
            .render(120 * scale, 80 * scale, f64::from(scale))
            .unwrap();
        let at = |x, y| pixel(&frame, x * scale, y * scale);
        assert_eq!(at(0, 0), [18, 52, 86, 255]);
        assert_eq!(at(15, 12), [171, 205, 239, 255]);
        assert_eq!(at(8, 18), [18, 52, 86, 255]);
        assert_eq!(at(15, 18), [255, 0, 0, 255]);
        assert_eq!(at(25, 22), [0, 255, 0, 255]);
        assert_eq!(at(89, 22), [0, 255, 0, 255]);
        assert_eq!(at(90, 22), [18, 52, 86, 255]);
    }
}

#[test]
fn pressed_button_darkens_its_custom_background_and_restores_it_on_release() {
    let mut runtime = runtime(
        r##"{
        "metadata":{"version":"0.1.0"}, "root":"0",
        "elements":{"0":{"position":{"x":0,"y":0},"size":{"width":40,"height":40},"kind":{"type":"button","label":"","font_size":16,"background":"#3366cc"}}}
    }"##,
    );
    assert_eq!(
        pixel(&runtime.render(40, 40, 1.0).unwrap(), 20, 20),
        [51, 102, 204, 255]
    );
    runtime.handle_input(Input::PointerPressed { x: 20.0, y: 20.0 });
    while runtime.next_event().is_some() {}
    let pressed = runtime.render(40, 40, 1.0).unwrap();
    assert_eq!(pixel(&pressed, 20, 20), [41, 83, 166, 255]);
    assert_eq!(pixel(&pressed, 0, 0), [0, 0, 0, 255]);
    runtime.handle_input(Input::PointerReleased { x: 20.0, y: 20.0 });
    while runtime.next_event().is_some() {}
    assert_eq!(
        pixel(&runtime.render(40, 40, 1.0).unwrap(), 20, 20),
        [51, 102, 204, 255]
    );
}

#[test]
fn text_and_control_glyphs_use_the_configured_color() {
    for kind in [
        r##"{"type":"text","text":"X","font_size":24,"color":"#cc3366"}"##,
        r##"{"type":"button","label":"X","font_size":24,"color":"#cc3366"}"##,
        r##"{"type":"text_input","value":"X","font_size":24,"color":"#cc3366","background":"#112233"}"##,
        r##"{"type":"text_input","value":"","placeholder":"X","font_size":24,"color":"#cc3366","background":"#112233"}"##,
    ] {
        let json = format!(
            r#"{{"metadata":{{"version":"0.1.0","default_font":"color-test.ttf"}},"root":"0","elements":{{"0":{{"position":{{"x":0,"y":0}},"size":{{"width":80,"height":40}},"kind":{kind}}}}}}}"#
        );
        let mut runtime = runtime(&json);
        for scale in [1, 2] {
            let frame = runtime
                .render(80 * scale, 40 * scale, f64::from(scale))
                .unwrap();
            assert!(
                frame
                    .rgba()
                    .as_chunks::<4>()
                    .0
                    .contains(&[204, 51, 102, 255]),
                "missing colored glyph for {kind}"
            );
            if kind.contains("text_input") {
                assert_eq!(pixel(&frame, 2 * scale, 2 * scale), [17, 34, 51, 255]);
            }
        }
    }
}

#[test]
fn mutations_recolor_containers_and_text_without_changing_structure() {
    let mut runtime = runtime(
        r#"{
        "metadata":{"version":"0.1.0","default_font":"color-test.ttf"}, "root":"0",
        "elements":{
            "0":{"position":{"x":0,"y":0},"size":{"width":80,"height":40},"kind":{"type":"container","children":["1"]}},
            "1":{"position":{"x":5,"y":5},"size":{"width":60,"height":30},"kind":{"type":"text","text":"X","font_size":24}}
        }
    }"#,
    );
    let before = runtime.render(80, 40, 1.0).unwrap();
    assert_eq!(pixel(&before, 0, 0), [255, 255, 255, 255]);
    assert!(before.rgba().as_chunks::<4>().0.contains(&[0, 0, 0, 255]));
    let mut container = runtime.document().data().elements[&0].kind.clone();
    let ElementKind::Container { background, .. } = &mut container else {
        unreachable!()
    };
    *background = Some(Color::rgb(0, 0, 128));
    let mut text = runtime.document().data().elements[&1].kind.clone();
    let ElementKind::Text { color, .. } = &mut text else {
        unreachable!()
    };
    *color = Some(Color::WHITE);
    runtime
        .apply(&[
            MutationOp::Update {
                id: 0,
                position: None,
                size: None,
                kind: Some(container),
            },
            MutationOp::Update {
                id: 1,
                position: None,
                size: None,
                kind: Some(text),
            },
        ])
        .unwrap();
    let after = runtime.render(80, 40, 1.0).unwrap();
    assert_eq!(pixel(&after, 0, 0), [0, 0, 128, 255]);
    assert!(
        after
            .rgba()
            .as_chunks::<4>()
            .0
            .contains(&[255, 255, 255, 255])
    );
    assert_eq!(runtime.document().parent(1), Some(0));
}
