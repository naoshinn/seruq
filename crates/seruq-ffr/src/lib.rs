//! The Foreign Function Runner: resolves the function names stored in UCR
//! actions and calls the functions an application supplies for them.
//!
//! A function reads the event and the current document and returns a
//! [`Mutation`]. It never changes the document itself; the application applies
//! the mutation, normally before it processes the next event.

use std::{collections::HashMap, fmt};

use seruq_ucr::{ElementId, Event, Mutation, Timing, Ucr};

/// A failure reported by a function.
pub type FunctionError = Box<dyn std::error::Error + Send + Sync>;

/// Supplies the functions behind function names.
///
/// [`Functions`] implements this for Rust closures. Hosts for other languages
/// can implement it to run their own functions through the same [`call`].
pub trait FunctionHost {
    fn contains(&self, function_name: &str) -> bool;

    /// Run a function. Returns `None` if this host has no function with that name.
    fn invoke(
        &mut self,
        function_name: &str,
        event: &Event,
        document: &Ucr,
    ) -> Option<Result<Mutation, FunctionError>>;
}

type Function = Box<dyn FnMut(&Event, &Ucr) -> Result<Mutation, FunctionError>>;

/// Rust closures registered under function names.
#[derive(Default)]
pub struct Functions {
    functions: HashMap<String, Function>,
}

impl Functions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function, replacing any function already registered under the name.
    pub fn register(
        &mut self,
        function_name: impl Into<String>,
        function: impl FnMut(&Event, &Ucr) -> Result<Mutation, FunctionError> + 'static,
    ) -> &mut Self {
        self.functions
            .insert(function_name.into(), Box::new(function));
        self
    }
}

impl FunctionHost for Functions {
    fn contains(&self, function_name: &str) -> bool {
        self.functions.contains_key(function_name)
    }

    fn invoke(
        &mut self,
        function_name: &str,
        event: &Event,
        document: &Ucr,
    ) -> Option<Result<Mutation, FunctionError>> {
        self.functions
            .get_mut(function_name)
            .map(|function| function(event, document))
    }
}

/// Why an action could not produce a mutation. The document is never changed by a failed call.
#[derive(Debug)]
pub struct CallError {
    pub element: ElementId,
    pub timing: Timing,
    pub function_name: String,
    pub reason: CallErrorReason,
}

#[derive(Debug)]
pub enum CallErrorReason {
    /// The host has no function with the action's name.
    UnknownFunction,
    /// The function ran and reported a failure.
    Function(FunctionError),
}

impl fmt::Display for CallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            element,
            timing,
            function_name,
            reason,
        } = self;
        match reason {
            CallErrorReason::UnknownFunction => write!(
                formatter,
                "unknown function {function_name:?} for {timing:?} on element {element}"
            ),
            CallErrorReason::Function(error) => write!(
                formatter,
                "function {function_name:?} failed for {timing:?} on element {element}: {error}"
            ),
        }
    }
}

impl std::error::Error for CallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            CallErrorReason::UnknownFunction => None,
            CallErrorReason::Function(error) => Some(error.as_ref()),
        }
    }
}

/// Call the function that the event's target has for the event's timing.
///
/// Returns `Ok(None)` without calling anything if the target is no longer
/// reachable from the root or has no action for the timing. The returned
/// mutation is not applied; pass it to [`Ucr::apply`] before handling the next event.
pub fn call(
    host: &mut impl FunctionHost,
    document: &Ucr,
    event: &Event,
) -> Result<Option<Mutation>, CallError> {
    if !document.is_reachable(event.target) {
        return Ok(None);
    }
    let timing = event.timing();
    let Some(action) = document.data().elements[&event.target].actions.get(&timing) else {
        return Ok(None);
    };
    let error = |reason| CallError {
        element: event.target,
        timing,
        function_name: action.function_name.clone(),
        reason,
    };
    match host.invoke(&action.function_name, event, document) {
        Some(Ok(mutation)) => Ok(Some(mutation)),
        Some(Err(source)) => Err(error(CallErrorReason::Function(source))),
        None => Err(error(CallErrorReason::UnknownFunction)),
    }
}

/// An action whose function name the host cannot resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub element: ElementId,
    pub timing: Timing,
    pub function_name: String,
}

/// Every action on a reachable element that names a function the host does
/// not have, ordered by element ID and then timing.
///
/// Run this after loading a document to find missing functions before any event occurs.
pub fn unresolved(host: &impl FunctionHost, document: &Ucr) -> Vec<Unresolved> {
    let mut unresolved: Vec<Unresolved> = document
        .data()
        .elements
        .iter()
        .filter(|(id, _)| document.is_reachable(**id))
        .flat_map(|(&element, data)| {
            data.actions
                .iter()
                .filter(|(_, call)| !host.contains(&call.function_name))
                .map(move |(&timing, call)| Unresolved {
                    element,
                    timing,
                    function_name: call.function_name.clone(),
                })
        })
        .collect();
    unresolved.sort_by_key(|entry| (entry.element, entry.timing));
    unresolved
}
