use std::collections::{HashMap, HashSet, hash_map::Entry};

use crate::{ElementKind, UcrDraft, ucr::ElementId};

#[derive(Debug, Clone, PartialEq)]
pub struct Ucr {
    pub(crate) data: UcrDraft,
    // Parent of every element reachable from the root, except the root itself.
    pub(crate) parents: HashMap<ElementId, ElementId>,
}

impl Ucr {
    pub fn data(&self) -> &UcrDraft {
        &self.data
    }

    pub fn into_data(self) -> UcrDraft {
        self.data
    }

    /// Whether the element exists and is the root or one of its descendants.
    pub fn is_reachable(&self, id: ElementId) -> bool {
        id == self.data.root || self.parents.contains_key(&id)
    }

    /// The parent Container of an element reachable from the root.
    pub fn parent(&self, id: ElementId) -> Option<ElementId> {
        self.parents.get(&id).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationWarning {
    UnusedElement { id: ElementId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    RootNotFound {
        root: ElementId,
    },
    ElementNotFound {
        parent: ElementId,
        child: ElementId,
    },
    /// Any reference to the root from the reachable structure, including cycles through it.
    RootHasParent {
        parent: ElementId,
    },
    Cycle {
        parent: ElementId,
        child: ElementId,
    },
    DuplicateReference {
        parent: ElementId,
        child: ElementId,
    },
}

enum Step {
    Enter(ElementId),
    Exit(ElementId),
}

impl ValidationReport {
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    pub fn warnings(&self) -> &[ValidationWarning] {
        &self.warnings
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }
}

impl UcrDraft {
    pub fn build(self) -> Result<(Ucr, Vec<ValidationWarning>), ValidationReport> {
        let (report, parents) = self.traverse();

        if report.has_errors() {
            Err(report)
        } else {
            Ok((
                Ucr {
                    data: self,
                    parents,
                },
                report.warnings,
            ))
        }
    }

    pub fn validate(&self) -> ValidationReport {
        self.traverse().0
    }

    fn traverse(&self) -> (ValidationReport, HashMap<ElementId, ElementId>) {
        let mut errors: Vec<ValidationError> = Vec::new();

        let elements = &self.elements;
        // The first Container that references each child claims it.
        let mut parents: HashMap<ElementId, ElementId> = HashMap::new();
        let mut visiting: HashSet<ElementId> = HashSet::new();
        // DFS stack. Only existing, newly claimed elements are entered.
        let mut stack = Vec::new();

        if elements.contains_key(&self.root) {
            stack.push(Step::Enter(self.root));
        } else {
            errors.push(ValidationError::RootNotFound { root: self.root });
        }

        while let Some(step) = stack.pop() {
            let parent = match step {
                Step::Enter(id) => id,
                Step::Exit(id) => {
                    visiting.remove(&id);
                    continue;
                }
            };

            visiting.insert(parent);
            // Push Exit before the children so it runs after their traversal (LIFO).
            stack.push(Step::Exit(parent));

            let ElementKind::Container { children } = &elements[&parent].kind else {
                continue;
            };
            let first_child = stack.len();
            for &child in children {
                if child == self.root {
                    errors.push(ValidationError::RootHasParent { parent });
                } else if visiting.contains(&child) {
                    errors.push(ValidationError::Cycle { parent, child });
                } else if !elements.contains_key(&child) {
                    errors.push(ValidationError::ElementNotFound { parent, child });
                } else if let Entry::Vacant(entry) = parents.entry(child) {
                    entry.insert(parent);
                    stack.push(Step::Enter(child));
                } else {
                    errors.push(ValidationError::DuplicateReference { parent, child });
                }
            }
            // Reverse the pushed children to visit them in their original order.
            stack[first_child..].reverse();
        }

        let mut unused: Vec<ElementId> = elements
            .keys()
            .copied()
            .filter(|id| *id != self.root && !parents.contains_key(id))
            .collect();
        unused.sort_unstable();
        let warnings = unused
            .into_iter()
            .map(|id| ValidationWarning::UnusedElement { id })
            .collect();

        (ValidationReport { errors, warnings }, parents)
    }
}
