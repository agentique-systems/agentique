//! Which blocks fit a port: found with the language's own rule
//! (`Semantics::ports_fit`, the `incompatible-ends` rule), never a second
//! copy of it. Blocks outside the project are tried as they would be once
//! used: copied, with what they need, into a scratch copy of the project, so
//! that shared definitions (the same `Library::Interfaces::RequestPort`)
//! are the same element there.

use crate::copy::{Resolution, apply_to_tree, closure, path_of, plan_copy};
use crate::{Index, Library, Scope};
use agq_language::{ElementId, ElementKind, Semantics, Tree};
use std::collections::HashMap;

/// A block with ports that fit: the block (an index into the index) and
/// the names of its fitting ports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fit {
    pub block: usize,
    pub ports: Vec<String>,
}

impl Library {
    /// Blocks of `index` (part definitions) with a port that can be
    /// connected to `port`, a port of the project: facing it, or, with
    /// `passes_on`, taking over what `port` passes in (the block would go
    /// inside the part that owns `port`).
    pub fn compatible(
        &self,
        index: &Index,
        project: &Tree,
        port: ElementId,
        passes_on: bool,
    ) -> Vec<Fit> {
        let mut scratch = project.clone();
        let mut candidates: Vec<(usize, Vec<(String, ElementId)>)> = Vec::new();
        for (i, block) in index.blocks().iter().enumerate() {
            if block.kind != ElementKind::PartDef || block.ports.is_empty() || block.is_abstract {
                continue;
            }
            if block.reference.scope == Scope::Project {
                candidates.push((
                    i,
                    block
                        .ports
                        .iter()
                        .map(|p| (p.name.clone(), p.element))
                        .collect(),
                ));
                continue;
            }
            if index.project_copy(block).is_some() {
                continue;
            }
            let Some(map) = self.copy_into(&mut scratch, block.reference.clone()) else {
                continue;
            };
            candidates.push((
                i,
                block
                    .ports
                    .iter()
                    .filter_map(|p| Some((p.name.clone(), *map.get(&p.element)?)))
                    .collect(),
            ));
        }
        let semantics = Semantics::new(&scratch);
        candidates
            .into_iter()
            .filter_map(|(block, ports)| {
                let fitting: Vec<String> = ports
                    .into_iter()
                    .filter(|(_, candidate)| {
                        semantics.ports_fit(port, *candidate, passes_on).is_ok()
                    })
                    .map(|(name, _)| name)
                    .collect();
                (!fitting.is_empty()).then_some(Fit {
                    block,
                    ports: fitting,
                })
            })
            .collect()
    }

    /// The ports among `ports` (ports of the project) that one of the
    /// block's ports could face: marked while the block is dragged over the
    /// Surface.
    pub fn fitting_ports(
        &self,
        index: &Index,
        block: usize,
        project: &Tree,
        ports: &[ElementId],
    ) -> Vec<ElementId> {
        let Some(summary) = index.get(block) else {
            return Vec::new();
        };
        if summary.ports.is_empty() {
            return Vec::new();
        }
        let mut scratch = project.clone();
        let own: Vec<ElementId> = if summary.reference.scope == Scope::Project {
            summary.ports.iter().map(|p| p.element).collect()
        } else {
            let Some(map) = self.copy_into(&mut scratch, summary.reference.clone()) else {
                return Vec::new();
            };
            summary
                .ports
                .iter()
                .filter_map(|p| map.get(&p.element).copied())
                .collect()
        };
        let semantics = Semantics::new(&scratch);
        ports
            .iter()
            .copied()
            .filter(|port| {
                own.iter()
                    .any(|candidate| semantics.ports_fit(*port, *candidate, false).is_ok())
            })
            .collect()
    }

    /// Copies a block with what it needs into `scratch` as using it would;
    /// the map from the block's elements to their copies, or `None` when it
    /// cannot be copied (missing definitions, a conflict).
    fn copy_into(
        &self,
        scratch: &mut Tree,
        reference: crate::BlockRef,
    ) -> Option<HashMap<ElementId, ElementId>> {
        let located = self.locate(&reference, None)?;
        if located.standard {
            return Some(HashMap::new());
        }
        let units = closure(located.tree, located.element).ok()?;
        let plan = plan_copy(
            located.tree,
            &units,
            scratch,
            &|unit| path_of(located.tree, unit),
            Resolution::Ask,
        )
        .ok()?;
        let map = plan.map.clone();
        apply_to_tree(scratch, plan.creations).ok()?;
        Some(map)
    }
}
