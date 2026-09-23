//! Bounded positive evidence for an already materialized specialization path.
use crate::{Completeness, KerMlQueries, QueryResult};
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, derived::PropertyState, provenance::FactKey, value::Value};
use std::collections::{BTreeSet, VecDeque};

#[cfg(test)]
#[path = "../tests/unit/specialization_witness.rs"]
mod tests;

impl KerMlQueries<'_> {
    /// Find positive evidence that `specific` already reaches `general` through
    /// canonical Specialization records. Only the selected path contributes
    /// endpoint/provenance evidence; unrelated outgoing edges do not.
    ///
    /// `None` means this bounded helper found no witness, never that the semantic
    /// specialization is absent. Virtual library/positional edges and feature
    /// chain terminals are intentionally not explored. Nodes with conjugation
    /// or pending specialization providers are not traversed, since conjugation
    /// replaces ordinary supertype edges. `exclude_implied` remains effective.
    pub fn canonical_specialization_witness(
        &self,
        specific: ElementId,
        general: ElementId,
    ) -> Option<QueryResult<()>> {
        let mut initial = self.result(());
        self.checked::<views::Type, _>(&mut initial, specific)?;
        self.checked::<views::Type, _>(&mut initial, general)?;
        let mut pending = VecDeque::from([(specific, initial)]);
        let mut seen = BTreeSet::new();
        while let Some((current, mut path)) = pending.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            if current == general {
                return (path.completeness == Completeness::Complete).then_some(path);
            }
            if self
                .context()
                .pending_specialization_scopes
                .contains(&current)
            {
                continue;
            }
            let conjugated = self.read_value(&mut path, current, p::TYPE_IS_CONJUGATED);
            if matches!(conjugated, Some(Value::Boolean(true)))
                || matches!(
                    self.model().property_state(current, p::TYPE_IS_CONJUGATED),
                    Ok(PropertyState::Incomplete(_) | PropertyState::Invalid(_))
                )
            {
                continue;
            }
            let conjugations = self.owned_relationships_of_type(current, c::CONJUGATION);
            if conjugations.completeness != Completeness::Complete || !conjugations.value.is_empty()
            {
                continue;
            }
            path.merge(conjugations);
            // The enumeration guides witness discovery, but absence is never
            // used as a conclusion. Its whole-population searches are therefore
            // not proof premises for the selected positive path.
            let mut discovery = self.result(());
            let candidates = self.incoming_source_relationships(
                &mut discovery,
                current,
                c::SPECIALIZATION,
                p::SPECIALIZATION_SPECIFIC,
            );
            for relationship in candidates {
                // These source roles additionally require owning-feature
                // semantics. Leave them to ordinary queries and materializers.
                if self.is(relationship, c::REFERENCE_SUBSETTING)
                    || self.is(relationship, c::CROSS_SUBSETTING)
                {
                    continue;
                }
                let mut edge = self.result(());
                let Some(view) = self.checked::<views::Specialization, _>(&mut edge, relationship)
                else {
                    continue;
                };
                self.property(&mut edge, relationship, p::SPECIALIZATION_SPECIFIC);
                if self.accept(&mut edge, relationship, view.specific()) != Some(current) {
                    continue;
                }
                if self.context().options.exclude_implied {
                    self.property(&mut edge, relationship, p::RELATIONSHIP_IS_IMPLIED);
                    if self.accept(&mut edge, relationship, view.is_implied()) != Some(false) {
                        continue;
                    }
                }
                self.property(&mut edge, relationship, p::SPECIALIZATION_GENERAL);
                let Some(target) = self.accept(&mut edge, relationship, view.general()) else {
                    continue;
                };
                self.fact(&mut edge, FactKey::Element(current));
                if self.checked::<views::Type, _>(&mut edge, target).is_none()
                    || edge.completeness != Completeness::Complete
                {
                    continue;
                }
                let mut next = path.clone();
                next.merge(edge);
                pending.push_back((target, next));
            }
        }
        None
    }
}
