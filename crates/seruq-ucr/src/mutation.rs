use std::{collections::BTreeSet, mem};

use crate::{Element, ElementId, ElementKind, Point, Size, Ucr, Value, ValueMap};

/// An ordered sequence of changes, applied to a [`Ucr`] as a single unit.
pub type Mutation = Vec<MutationOp>;

#[derive(Debug, Clone, PartialEq)]
pub enum MutationOp {
    /// Add a new element to `parent` at `index`. A Container must be inserted
    /// without children; build subtrees with further `Insert` operations.
    Insert {
        id: ElementId,
        element: Element,
        parent: ElementId,
        index: usize,
    },
    /// Remove an element together with all of its descendants.
    Remove { id: ElementId },
    /// Reparent or reorder an element. `index` is a position in the
    /// destination's children after the element has been detached.
    Move {
        id: ElementId,
        parent: ElementId,
        index: usize,
    },
    /// Replace the given fields. `kind` must keep the element's type and
    /// cannot target a Container, whose children change through the other operations.
    Update {
        id: ElementId,
        position: Option<Point>,
        size: Option<Size>,
        kind: Option<ElementKind>,
    },
    /// Set one key of the document's `state`, replacing any existing value.
    SetState { key: String, value: Value },
    /// Remove one key of the document's `state`. An absent key is not an error.
    RemoveState { key: String },
    /// Set one key of an element's `data`, replacing any existing value.
    SetData {
        id: ElementId,
        key: String,
        value: Value,
    },
    /// Remove one key of an element's `data`. An absent key is not an error.
    RemoveData { id: ElementId, key: String },
}

/// The elements and state keys affected by a successfully applied [`Mutation`].
///
/// Containers whose `children` changed are not listed unless they were
/// themselves inserted, moved, or updated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    inserted: BTreeSet<ElementId>,
    removed: BTreeSet<ElementId>,
    updated: BTreeSet<ElementId>,
    state: BTreeSet<String>,
}

impl Applied {
    pub fn inserted(&self) -> &BTreeSet<ElementId> {
        &self.inserted
    }

    /// Elements that existed before the mutation and were removed by it.
    /// An ID reused by a later `Insert` is also listed in `inserted`.
    pub fn removed(&self) -> &BTreeSet<ElementId> {
        &self.removed
    }

    /// Surviving elements that were moved or updated, including changes to their `data`.
    pub fn updated(&self) -> &BTreeSet<ElementId> {
        &self.updated
    }

    /// Keys of the document's `state` that were set or removed.
    pub fn state(&self) -> &BTreeSet<String> {
        &self.state
    }
}

/// The first failing operation. The document is left unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationError {
    pub index: usize,
    pub reason: MutationErrorReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationErrorReason {
    DuplicateId {
        id: ElementId,
    },
    ElementNotFound {
        id: ElementId,
    },
    /// The element exists but is not part of the structure reachable from the root.
    Unreachable {
        id: ElementId,
    },
    NotContainer {
        id: ElementId,
    },
    IndexOutOfBounds {
        parent: ElementId,
        index: usize,
        len: usize,
    },
    InsertedContainerHasChildren {
        id: ElementId,
    },
    /// The root cannot be removed or moved.
    TargetsRoot,
    WouldCycle {
        id: ElementId,
        parent: ElementId,
    },
    KindMismatch {
        id: ElementId,
    },
    ContainerUpdate {
        id: ElementId,
    },
}

impl Ucr {
    /// Apply every operation in order, or none of them.
    ///
    /// Each operation is checked against the document as changed by the
    /// preceding ones, so the result is valid without a full revalidation.
    pub fn apply(&mut self, mutation: &[MutationOp]) -> Result<Applied, MutationError> {
        // Work on a copy so that a failure leaves the document unchanged.
        let mut next = self.clone();
        let mut applied = Applied::default();

        for (index, op) in mutation.iter().enumerate() {
            next.apply_op(op, &mut applied)
                .map_err(|reason| MutationError { index, reason })?;
        }

        *self = next;
        Ok(applied)
    }

    fn apply_op(
        &mut self,
        op: &MutationOp,
        applied: &mut Applied,
    ) -> Result<(), MutationErrorReason> {
        match op {
            MutationOp::Insert {
                id,
                element,
                parent,
                index,
            } => {
                let (id, parent, index) = (*id, *parent, *index);
                if self.data.elements.contains_key(&id) {
                    return Err(MutationErrorReason::DuplicateId { id });
                }
                if matches!(&element.kind, ElementKind::Container { children } if !children.is_empty())
                {
                    return Err(MutationErrorReason::InsertedContainerHasChildren { id });
                }
                let children = self.children_mut(parent)?;
                if index > children.len() {
                    return Err(MutationErrorReason::IndexOutOfBounds {
                        parent,
                        index,
                        len: children.len(),
                    });
                }
                children.insert(index, id);
                self.data.elements.insert(id, element.clone());
                self.parents.insert(id, parent);
                applied.inserted.insert(id);
            }
            MutationOp::Remove { id } => {
                let id = *id;
                self.check_attached(id)?;
                let Some(parent) = self.parents.get(&id).copied() else {
                    return Err(MutationErrorReason::TargetsRoot);
                };
                self.children_mut(parent)?.retain(|child| *child != id);

                let mut stack = vec![id];
                while let Some(id) = stack.pop() {
                    self.parents.remove(&id);
                    if let Some(Element {
                        kind: ElementKind::Container { children },
                        ..
                    }) = self.data.elements.remove(&id)
                    {
                        stack.extend(children);
                    }
                    // An element inserted by this mutation never existed for the caller.
                    if !applied.inserted.remove(&id) {
                        applied.removed.insert(id);
                    }
                    applied.updated.remove(&id);
                }
            }
            MutationOp::Move { id, parent, index } => {
                let (id, parent, index) = (*id, *parent, *index);
                self.check_attached(id)?;
                let Some(old_parent) = self.parents.get(&id).copied() else {
                    return Err(MutationErrorReason::TargetsRoot);
                };
                let len = self.children_mut(parent)?.len() - usize::from(old_parent == parent);

                // The destination must not be the element itself or one of its descendants.
                let mut ancestor = Some(parent);
                while let Some(current) = ancestor {
                    if current == id {
                        return Err(MutationErrorReason::WouldCycle { id, parent });
                    }
                    ancestor = self.parents.get(&current).copied();
                }
                if index > len {
                    return Err(MutationErrorReason::IndexOutOfBounds { parent, index, len });
                }

                self.children_mut(old_parent)?.retain(|child| *child != id);
                self.children_mut(parent)?.insert(index, id);
                self.parents.insert(id, parent);
                if !applied.inserted.contains(&id) {
                    applied.updated.insert(id);
                }
            }
            MutationOp::Update {
                id,
                position,
                size,
                kind,
            } => {
                let id = *id;
                self.check_attached(id)?;
                let Some(element) = self.data.elements.get_mut(&id) else {
                    return Err(MutationErrorReason::ElementNotFound { id });
                };
                if let Some(kind) = kind {
                    if matches!(kind, ElementKind::Container { .. }) {
                        return Err(MutationErrorReason::ContainerUpdate { id });
                    }
                    if mem::discriminant(kind) != mem::discriminant(&element.kind) {
                        return Err(MutationErrorReason::KindMismatch { id });
                    }
                    element.kind = kind.clone();
                }
                if let Some(position) = position {
                    element.position = *position;
                }
                if let Some(size) = size {
                    element.size = *size;
                }
                if !applied.inserted.contains(&id) {
                    applied.updated.insert(id);
                }
            }
            MutationOp::SetState { key, value } => {
                self.data.state.insert(key.clone(), value.clone());
                applied.state.insert(key.clone());
            }
            MutationOp::RemoveState { key } => {
                if self.data.state.remove(key).is_some() {
                    applied.state.insert(key.clone());
                }
            }
            MutationOp::SetData { id, key, value } => {
                self.data_mut(*id)?.insert(key.clone(), value.clone());
                if !applied.inserted.contains(id) {
                    applied.updated.insert(*id);
                }
            }
            MutationOp::RemoveData { id, key } => {
                if self.data_mut(*id)?.remove(key).is_some() && !applied.inserted.contains(id) {
                    applied.updated.insert(*id);
                }
            }
        }
        Ok(())
    }

    fn check_attached(&self, id: ElementId) -> Result<(), MutationErrorReason> {
        if !self.data.elements.contains_key(&id) {
            Err(MutationErrorReason::ElementNotFound { id })
        } else if id != self.data.root && !self.parents.contains_key(&id) {
            Err(MutationErrorReason::Unreachable { id })
        } else {
            Ok(())
        }
    }

    fn data_mut(&mut self, id: ElementId) -> Result<&mut ValueMap, MutationErrorReason> {
        self.check_attached(id)?;
        match self.data.elements.get_mut(&id) {
            Some(element) => Ok(&mut element.data),
            None => Err(MutationErrorReason::ElementNotFound { id }),
        }
    }

    fn children_mut(&mut self, id: ElementId) -> Result<&mut Vec<ElementId>, MutationErrorReason> {
        self.check_attached(id)?;
        match self
            .data
            .elements
            .get_mut(&id)
            .map(|element| &mut element.kind)
        {
            Some(ElementKind::Container { children }) => Ok(children),
            _ => Err(MutationErrorReason::NotContainer { id }),
        }
    }
}

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod mutation_tests;
