use seruq_test_runtime::{Bounds, Placed, Runtime};
use seruq_ucr::{
    Element, ElementId, ElementKind, FontSize, Metadata, MutationOp, Point, Size, UcrDraft,
    ValueMap,
};

fn container(children: &[ElementId]) -> ElementKind {
    ElementKind::Container {
        children: children.to_vec(),
    }
}

// An empty label needs no font, so these tests run without font assets.
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

fn element(x: f32, y: f32, width: f32, height: f32, kind: ElementKind) -> Element {
    Element::new(
        Point::new(x, y).unwrap(),
        Size::new(width, height).unwrap(),
        kind,
    )
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

fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x,
        y,
        width,
        height,
    }
}

/// 0 (100,50 200x100): [1, 2]
///   1 (10,20 50x30) Button
///   2 (150,60 100x100) Container: [3, 4]   -- clipped to 50x40 by the root
///     3 (-10,-10 30x30) Button             -- partly outside 2
///     4 (60,0 10x10) Button                -- inside 2 but outside the root
fn nested() -> Runtime {
    runtime([
        (0, element(100.0, 50.0, 200.0, 100.0, container(&[1, 2]))),
        (1, element(10.0, 20.0, 50.0, 30.0, button())),
        (2, element(150.0, 60.0, 100.0, 100.0, container(&[3, 4]))),
        (3, element(-10.0, -10.0, 30.0, 30.0, button())),
        (4, element(60.0, 0.0, 10.0, 10.0, button())),
    ])
}

#[test]
fn layout_accumulates_positions_in_paint_order() {
    let placed = nested().layout();
    assert_eq!(
        placed.iter().map(|placed| placed.id).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(placed[0].bounds, bounds(100.0, 50.0, 200.0, 100.0));
    assert_eq!(placed[1].bounds, bounds(110.0, 70.0, 50.0, 30.0));
    assert_eq!(placed[2].bounds, bounds(250.0, 110.0, 100.0, 100.0));
    assert_eq!(placed[3].bounds, bounds(240.0, 100.0, 30.0, 30.0));
    assert_eq!(placed[4].bounds, bounds(310.0, 110.0, 10.0, 10.0));
}

#[test]
fn layout_clips_to_every_ancestor_container() {
    let placed = nested().layout();
    assert_eq!(placed[0].visible, Some(placed[0].bounds));
    assert_eq!(placed[1].visible, Some(placed[1].bounds));
    assert_eq!(placed[2].visible, Some(bounds(250.0, 110.0, 50.0, 40.0)));
    assert_eq!(placed[3].visible, Some(bounds(250.0, 110.0, 20.0, 20.0)));
    assert_eq!(placed[4].visible, None);
}

#[test]
fn layout_lists_children_of_fully_clipped_and_empty_containers() {
    let runtime = runtime([
        (0, element(0.0, 0.0, 100.0, 100.0, container(&[1, 3, 5]))),
        (1, element(200.0, 0.0, 50.0, 50.0, container(&[2]))),
        (2, element(-200.0, 0.0, 10.0, 10.0, button())),
        (3, element(0.0, 0.0, 0.0, 50.0, container(&[4]))),
        (4, element(0.0, 0.0, 10.0, 10.0, button())),
        (5, element(10.0, 10.0, 0.0, 0.0, button())),
    ]);
    let placed = runtime.layout();
    assert_eq!(
        placed.iter().map(|placed| placed.id).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5]
    );
    // Element 2 lies inside the root's bounds but its parent is fully clipped.
    assert_eq!(placed[2].bounds, bounds(0.0, 0.0, 10.0, 10.0));
    assert!(
        placed[1..]
            .iter()
            .all(|Placed { visible, .. }| visible.is_none())
    );
    assert_eq!(runtime.hit_test(5.0, 5.0), None);
}

#[test]
fn hit_test_uses_half_open_regions() {
    let runtime = nested();
    assert_eq!(runtime.hit_test(110.0, 70.0), Some(1));
    assert_eq!(runtime.hit_test(159.999, 99.999), Some(1));
    assert_eq!(runtime.hit_test(160.0, 80.0), None);
    assert_eq!(runtime.hit_test(120.0, 100.0), None);
    assert_eq!(runtime.hit_test(109.999, 80.0), None);
}

#[test]
fn hit_test_ignores_containers_and_clipped_regions() {
    let runtime = nested();
    // Inside the root Container only.
    assert_eq!(runtime.hit_test(105.0, 55.0), None);
    // Inside Container 2 only.
    assert_eq!(runtime.hit_test(290.0, 140.0), None);
    // The visible corner of 3.
    assert_eq!(runtime.hit_test(260.0, 120.0), Some(3));
    // Parts of 3 clipped by its parent, and 4 clipped by the root.
    assert_eq!(runtime.hit_test(245.0, 105.0), None);
    assert_eq!(runtime.hit_test(315.0, 115.0), None);
    // Outside everything.
    assert_eq!(runtime.hit_test(0.0, 0.0), None);
    assert_eq!(runtime.hit_test(f64::NAN, 80.0), None);
}

#[test]
fn hit_test_returns_frontmost_element() {
    // 1 Button behind; 2 Container in front holding 3 Text that overlaps half of 1.
    let runtime = runtime([
        (0, element(0.0, 0.0, 200.0, 100.0, container(&[1, 2]))),
        (1, element(0.0, 0.0, 100.0, 100.0, button())),
        (2, element(0.0, 0.0, 200.0, 100.0, container(&[3]))),
        (3, element(50.0, 0.0, 100.0, 100.0, text())),
    ]);
    // A transparent Container does not hide the Button behind it.
    assert_eq!(runtime.hit_test(25.0, 50.0), Some(1));
    // A non-interactive leaf in front does.
    assert_eq!(runtime.hit_test(75.0, 50.0), Some(3));
    assert_eq!(runtime.hit_test(125.0, 50.0), Some(3));
}

#[test]
fn non_container_root_is_placed_at_its_position() {
    let runtime = runtime([(0, element(-5.0, 10.0, 20.0, 20.0, button()))]);
    assert_eq!(
        runtime.layout(),
        [Placed {
            id: 0,
            bounds: bounds(-5.0, 10.0, 20.0, 20.0),
            visible: Some(bounds(-5.0, 10.0, 20.0, 20.0)),
        }]
    );
    assert_eq!(runtime.hit_test(-5.0, 10.0), Some(0));
}

#[test]
fn applied_mutation_changes_layout_and_hit_test() {
    let mut runtime = nested();
    let applied = runtime
        .apply(&[
            MutationOp::Update {
                id: 1,
                position: Some(Point::new(100.0, 20.0).unwrap()),
                size: None,
                kind: None,
            },
            MutationOp::Remove { id: 2 },
        ])
        .unwrap();
    assert!(applied.updated().contains(&1));
    assert!(applied.removed().contains(&3));
    assert_eq!(runtime.hit_test(120.0, 80.0), None);
    assert_eq!(runtime.hit_test(210.0, 80.0), Some(1));
    assert_eq!(runtime.hit_test(260.0, 120.0), None);
    assert_eq!(runtime.layout().len(), 2);

    assert!(runtime.apply(&[MutationOp::Remove { id: 0 }]).is_err());
    assert_eq!(runtime.layout().len(), 2);
}

fn pixel(frame: &seruq_test_runtime::Frame, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * frame.width() + x) * 4) as usize;
    frame.rgba()[offset..offset + 4].try_into().unwrap()
}

#[test]
fn render_paints_only_visible_regions() {
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const BLACK: [u8; 4] = [0, 0, 0, 255];

    for scale in [1, 2] {
        let frame = nested()
            .render(400 * scale, 200 * scale, f64::from(scale))
            .unwrap();
        let at = |x: u32, y: u32| pixel(&frame, x * scale, y * scale);
        // Button 1: border, then white interior.
        assert_eq!(at(110, 70), BLACK);
        assert_eq!(at(159, 99), BLACK);
        assert_eq!(at(135, 85), WHITE);
        // Outside Button 1.
        assert_eq!(at(109, 70), WHITE);
        assert_eq!(at(160, 85), WHITE);
        // Button 3 is clipped to its parent: its border starts at x = 240, outside the clip.
        assert_eq!(at(245, 105), WHITE);
        assert_eq!(at(260, 129), BLACK);
        assert_eq!(at(269, 120), BLACK);
        assert_eq!(at(260, 120), WHITE);
        // Button 4 is clipped away by the root.
        assert_eq!(at(310, 110), WHITE);
    }
}
