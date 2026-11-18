use std::collections::BTreeMap;

use crate::ElementId;

/// The moments at which an element can trigger an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Timing {
    Pushed,
    Released,
    Activated,
    Focused,
    Unfocused,
    ValueChanged,
}

/// A description of a function to call through FFR. It does not contain the function itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfrCall {
    pub function_name: String,
}

impl FfrCall {
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
        }
    }
}

/// At most one call per timing.
pub type Actions = BTreeMap<Timing, FfrCall>;

/// Something that happened to an element, passed to the function it triggers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub target: ElementId,
    pub kind: EventKind,
}

/// A [`Timing`] together with the data that belongs to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Pushed,
    Released,
    Activated,
    Focused,
    Unfocused,
    ValueChanged { value: String },
}

impl Event {
    pub fn new(target: ElementId, kind: EventKind) -> Self {
        Self { target, kind }
    }

    pub fn timing(&self) -> Timing {
        match self.kind {
            EventKind::Pushed => Timing::Pushed,
            EventKind::Released => Timing::Released,
            EventKind::Activated => Timing::Activated,
            EventKind::Focused => Timing::Focused,
            EventKind::Unfocused => Timing::Unfocused,
            EventKind::ValueChanged { .. } => Timing::ValueChanged,
        }
    }
}
