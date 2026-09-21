//! Scoped producer dependencies. Lexical package environments stay immutable and
//! available, while referenced Types and their semantic owners are scheduled.
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{QueryInvalidationSet, QueryReadSet};
use agq_kernel::{ElementId, ModelView};
use std::collections::{BTreeSet, VecDeque};

fn is(model: &ModelView, id: ElementId, class: agq_kernel::MetaclassId) -> bool {
    model.element(id).is_some_and(|record| {
        model
            .registry()
            .is_subtype(record.metaclass(), class)
            .unwrap_or(false)
    })
}

/// Context anchors identify the validated standard library, but a Package anchor
/// is only a lookup environment. It does not request producers for every member.
/// Explicit seeds and structural references still expand Packages normally.
/// Concrete non-Package anchors retain their complete semantic owner closure.
pub fn subjects_with_context_anchors(
    model: &ModelView,
    seeds: impl IntoIterator<Item = ElementId>,
    anchors: impl IntoIterator<Item = ElementId>,
) -> Result<BTreeSet<ElementId>, String> {
    let mut selected: BTreeSet<_> = seeds.into_iter().collect();
    for anchor in anchors {
        if model.element(anchor).is_none() {
            return Err(format!("Missing slice context anchor {anchor}"));
        }
        if !is(model, anchor, c::PACKAGE) {
            selected.insert(anchor);
        }
    }
    subjects(model, selected)
}

/// Follow stored structural dependencies, together with enclosing semantic Types.
/// A return Feature brings its Function; an end brings its Connector; a nested
/// Feature brings its owning Type. A lexical Package does not bring all siblings.
/// Association navigation is already present in the kernel outgoing index.
pub fn subjects(
    model: &ModelView,
    seeds: impl IntoIterator<Item = ElementId>,
) -> Result<BTreeSet<ElementId>, String> {
    let mut selected: BTreeSet<_> = seeds.into_iter().collect();
    let mut pending: VecDeque<_> = selected.iter().copied().collect();
    while let Some(subject) = pending.pop_front() {
        if model.element(subject).is_none() {
            return Err(format!("Missing slice dependency {subject}"));
        }
        let owners = model
            .incoming(subject)
            .filter_map(|edge| match edge.property {
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT => Some(edge.source),
                p::ELEMENT_OWNED_RELATIONSHIP
                    if is(model, edge.source, c::TYPE)
                        || is(model, edge.source, c::RELATIONSHIP) =>
                {
                    Some(edge.source)
                }
                _ => None,
            });
        for owner in owners {
            if selected.insert(owner) {
                pending.push_back(owner);
            }
        }
        // A package import changes lookup availability, not producer ownership.
        // Actual semantic dependencies found through lookup are checked below.
        if is(model, subject, c::IMPORT) {
            continue;
        }
        for edge in model.outgoing(subject) {
            if !model
                .registry()
                .property(edge.property)
                .map_err(|e| e.to_string())?
                .derived
                && selected.insert(edge.target)
            {
                pending.push_back(edge.target);
            }
        }
    }
    Ok(selected)
}

/// A failed boundary is explicit evidence that the selected population cannot
/// yet claim a dependency-complete slice. It never seals a publication.
#[derive(Default)]
pub struct Boundary {
    pub missing_subjects: BTreeSet<ElementId>,
    pub unbounded_reads: usize,
    checked_read_subjects: BTreeSet<ElementId>,
}
impl Boundary {
    pub fn from_graph(model: &ModelView, population: &BTreeSet<ElementId>) -> Result<Self, String> {
        let required = subjects(model, population.iter().copied())?;
        Ok(Self {
            missing_subjects: required.difference(population).copied().collect(),
            unbounded_reads: 0,
            checked_read_subjects: BTreeSet::new(),
        })
    }

    /// Check the exact normalized positive/negative reads used by the producer
    /// scheduler. Package searches retain their complete declared environment;
    /// only Type/Feature/Function providers can contribute publication facts.
    /// A whole-model read requires the complete producer-capable Type population.
    /// Reuse this boundary only with the same final graph and population: shared
    /// dependency subjects are checked once across the whole audit.
    pub fn include_reads(
        &mut self,
        model: &ModelView,
        population: &BTreeSet<ElementId>,
        reads: &QueryReadSet,
    ) {
        self.include_elements(
            model,
            population,
            reads.bounded_elements(),
            reads.reads_entire_model(),
        );
    }

    /// Consume the closure scheduler's final dependency union without rerunning
    /// every producer solely to rediscover its read set.
    pub fn include_invalidation(
        &mut self,
        model: &ModelView,
        population: &BTreeSet<ElementId>,
        reads: &QueryInvalidationSet,
    ) {
        self.include_elements(
            model,
            population,
            reads.bounded_elements().iter().copied(),
            reads.reads_entire_model(),
        );
    }

    fn include_elements(
        &mut self,
        model: &ModelView,
        population: &BTreeSet<ElementId>,
        elements: impl IntoIterator<Item = ElementId>,
        entire_model: bool,
    ) {
        if entire_model {
            if self.unbounded_reads == 0 {
                self.missing_subjects.extend(
                    model
                        .elements()
                        .filter(|r| is(model, r.id(), c::TYPE) && !population.contains(&r.id()))
                        .map(|r| r.id()),
                );
            }
            self.unbounded_reads += 1;
        }
        let mut pending: Vec<_> = elements.into_iter().collect();
        while let Some(subject) = pending.pop() {
            if !self.checked_read_subjects.insert(subject) {
                continue;
            }
            if model.element(subject).is_none() {
                self.missing_subjects.insert(subject);
                continue;
            }
            if is(model, subject, c::TYPE) && !population.contains(&subject) {
                self.missing_subjects.insert(subject);
            }
            // A relationship read may depend on a producer on its semantic owner.
            // Walk containment, but stop at an ordinary lexical Namespace/Package.
            for edge in model.incoming(subject) {
                if edge.property == p::RELATIONSHIP_OWNED_RELATED_ELEMENT
                    || (edge.property == p::ELEMENT_OWNED_RELATIONSHIP
                        && (is(model, edge.source, c::TYPE)
                            || is(model, edge.source, c::RELATIONSHIP)))
                {
                    pending.push(edge.source);
                }
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.missing_subjects.is_empty()
    }
}
