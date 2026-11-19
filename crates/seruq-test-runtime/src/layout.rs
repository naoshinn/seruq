use seruq_ucr::{ElementId, ElementKind, Ucr};

/// A rectangle covering `[x, x + width)` by `[y, y + height)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    /// The overlap of two rectangles, or `None` if it is empty.
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }

    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    pub(crate) fn inset(self, amount: f64) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: (self.width - 2.0 * amount).max(0.0),
            height: (self.height - 2.0 * amount).max(0.0),
        }
    }

    pub(crate) fn scale(self, scale: f64) -> Self {
        Self {
            x: self.x * scale,
            y: self.y * scale,
            width: self.width * scale,
            height: self.height * scale,
        }
    }
}

/// An element's place in the viewport, in logical coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub id: ElementId,
    /// The element's own region. f64 keeps accumulated coordinates from overflowing f32.
    pub bounds: Bounds,
    /// The part of `bounds` left after clipping by every ancestor Container,
    /// or `None` if nothing remains. The viewport is not taken into account.
    pub visible: Option<Bounds>,
}

/// Every element reachable from the root, in paint order: later entries are in front.
pub(crate) fn layout(document: &Ucr) -> Vec<Placed> {
    let data = document.data();
    let mut placed = Vec::new();
    // `None` as the outer value means unclipped; as the inner value, fully clipped.
    let mut stack: Vec<(ElementId, f64, f64, Option<Option<Bounds>>)> =
        vec![(data.root, 0.0, 0.0, None)];
    while let Some((id, parent_x, parent_y, parent_clip)) = stack.pop() {
        // Ucr guarantees that every reachable reference exists and has a unique parent.
        let element = &data.elements[&id];
        let bounds = Bounds {
            x: parent_x + f64::from(element.position.x()),
            y: parent_y + f64::from(element.position.y()),
            width: f64::from(element.size.width()),
            height: f64::from(element.size.height()),
        };
        let visible = match parent_clip {
            Some(clip) => clip.and_then(|clip| clip.intersect(bounds)),
            // Intersecting with itself also rejects empty regions.
            None => bounds.intersect(bounds),
        };
        placed.push(Placed {
            id,
            bounds,
            visible,
        });
        if let ElementKind::Container { children, .. } = &element.kind {
            // Push in reverse so each subtree is completed in children order.
            stack.extend(
                children
                    .iter()
                    .rev()
                    .map(|&child| (child, bounds.x, bounds.y, Some(visible))),
            );
        }
    }
    placed
}

/// The frontmost element whose visible region contains the logical point.
///
/// Containers are transparent and never hit. Every other element type is a
/// target and hides the elements behind it.
pub(crate) fn hit_test(document: &Ucr, x: f64, y: f64) -> Option<ElementId> {
    let elements = &document.data().elements;
    layout(document)
        .into_iter()
        .rev()
        .find(|placed| {
            !matches!(elements[&placed.id].kind, ElementKind::Container { .. })
                && placed.visible.is_some_and(|visible| visible.contains(x, y))
        })
        .map(|placed| placed.id)
}
