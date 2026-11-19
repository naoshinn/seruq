use seruq_ucr::{Applied, ElementId, ElementKind, Event, EventKind, Ucr};

use crate::layout::hit_test;

/// Pointer input in logical coordinates, limited to the primary button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    PointerMoved {
        x: f64,
        y: f64,
    },
    PointerPressed {
        x: f64,
        y: f64,
    },
    PointerReleased {
        x: f64,
        y: f64,
    },
    /// The pointer left the viewport. A press in progress continues and can
    /// still be activated if the pointer returns before it is released.
    PointerLeft,
    /// The press in progress is abandoned, for example because the window lost focus.
    PointerCancelled,
}

struct Press {
    target: ElementId,
    /// The last known pointer position, or `None` after the pointer left the viewport.
    position: Option<(f64, f64)>,
}

/// Temporary input state. It is kept apart from the document and never stored in UCR.
#[derive(Default)]
pub(crate) struct Interaction {
    press: Option<Press>,
}

impl Interaction {
    pub fn handle(&mut self, document: &Ucr, input: Input) -> Vec<Event> {
        match input {
            Input::PointerPressed { x, y } => {
                // A second press cannot start while one is in progress.
                if self.press.is_some() {
                    return Vec::new();
                }
                let Some(target) = hit_test(document, x, y) else {
                    return Vec::new();
                };
                if !matches!(
                    document.data().elements[&target].kind,
                    ElementKind::Button { .. }
                ) {
                    return Vec::new();
                }
                self.press = Some(Press {
                    target,
                    position: Some((x, y)),
                });
                vec![Event::new(target, EventKind::Pushed)]
            }
            Input::PointerMoved { x, y } => {
                if let Some(press) = &mut self.press {
                    press.position = Some((x, y));
                }
                Vec::new()
            }
            Input::PointerLeft => {
                if let Some(press) = &mut self.press {
                    press.position = None;
                }
                Vec::new()
            }
            Input::PointerReleased { x, y } => {
                let Some(Press { target, .. }) = self.press.take() else {
                    return Vec::new();
                };
                let mut events = vec![Event::new(target, EventKind::Released)];
                // Releasing over another element, or over a part of the target
                // that is hidden or clipped, does not activate it.
                if hit_test(document, x, y) == Some(target) {
                    events.push(Event::new(target, EventKind::Activated));
                }
                events
            }
            Input::PointerCancelled => match self.press.take() {
                Some(Press { target, .. }) => vec![Event::new(target, EventKind::Released)],
                None => Vec::new(),
            },
        }
    }

    /// Drop a press whose target a mutation removed. No event is produced
    /// because there is no element left to receive it.
    pub fn reconcile(&mut self, document: &Ucr, applied: &Applied) {
        if self.press.as_ref().is_some_and(|press| {
            applied.removed().contains(&press.target) || !document.is_reachable(press.target)
        }) {
            self.press = None;
        }
    }

    /// The Button that is held down with the pointer still over it.
    pub fn pressed(&self, document: &Ucr) -> Option<ElementId> {
        let press = self.press.as_ref()?;
        let (x, y) = press.position?;
        (hit_test(document, x, y) == Some(press.target)).then_some(press.target)
    }
}
