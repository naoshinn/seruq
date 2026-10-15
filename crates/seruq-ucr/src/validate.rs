use std::collections::HashSet;

use crate::{ElementKind, UcrDraft, ucr::ElementId};

pub struct Ucr {
    data: UcrDraft,
}

impl Ucr {
    pub fn data(&self) -> &UcrDraft {
        &self.data
    }

    pub fn into_data(self) -> UcrDraft {
        self.data
    }
}

pub struct ValidationReport {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
}

pub enum ValidationWarning {
    UnusedElement,
}

pub enum ValidationError {
    RootHasParent,
    ElementNotFound,
    InvalidFontSize,
    Cycle,
    DuplicateReference,
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
        let report = self.validate();

        if report.has_errors() {
            Err(report)
        } else {
            Ok((Ucr { data: self }, report.warnings))
        }
    }

    pub fn validate(&self) -> ValidationReport {
        let mut warnings: Vec<ValidationWarning> = Vec::new();
        let mut errors: Vec<ValidationError> = Vec::new();

        let elements = &self.elements;
        let mut unvisited: HashSet<_> = elements.keys().collect();
        // DFS stack
        let mut stack = vec![Step::Enter(self.root)];
        let mut visiting: HashSet<ElementId> = HashSet::new();
        let mut visited: HashSet<ElementId> = HashSet::new();

        while let Some(step) = stack.pop() {
            let id = match step {
                Step::Enter(id) => id,
                Step::Exit(id) => {
                    visiting.remove(&id);
                    visited.insert(id);
                    continue;
                }
            };

            if visiting.contains(&id) {
                errors.push(ValidationError::Cycle);
                continue;
            }
            if visited.contains(&id) {
                errors.push(ValidationError::DuplicateReference);
                continue;
            }

            let Some(element) = elements.get(&id) else {
                errors.push(ValidationError::ElementNotFound);
                continue;
            };

            unvisited.remove(&id);
            visiting.insert(id);
            // Push Exit before the children so it runs after their traversal (LIFO).
            stack.push(Step::Exit(id));

            match &element.kind {
                ElementKind::Container { children } => {
                    if children.contains(&self.root) {
                        errors.push(ValidationError::RootHasParent);
                    }
                    // Push children in reverse order to visit them in their original order.
                    stack.extend(children.iter().rev().copied().map(Step::Enter));
                }
                ElementKind::Text { font_size, .. }
                | ElementKind::Button { font_size, .. }
                | ElementKind::TextInput { font_size, .. }
                    if !font_size.is_finite() || *font_size <= 0.0 =>
                {
                    errors.push(ValidationError::InvalidFontSize);
                }
                _ => {}
            }
        }

        if !unvisited.is_empty() {
            warnings.push(ValidationWarning::UnusedElement);
        }

        ValidationReport { errors, warnings }
    }
}
