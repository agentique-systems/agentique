//! Reference denotation belongs here, independent of any textual parser.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, MetaclassId};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedName {
    pub absolute: bool,
    pub segments: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved(ElementId),
    Unresolved,
    Ambiguous(Vec<ElementId>),
    WrongKind(ElementId),
    /// Imports, inherited lookup, or other unavailable evidence blocks denotation.
    Incomplete,
}
impl KerMlQueries<'_> {
    /// KerML 8.2.3.5: owned specialization references start in the owning
    /// Type's owning Namespace. This bounded query handles long declared names,
    /// lexical parents and public qualified traversal within one root namespace.
    /// Inheritance/imports at a searched scope explicitly block this slice.
    pub fn resolve_reference(
        &self,
        specific: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
    ) -> QueryResult<Resolution> {
        let mut out = self.result(Resolution::Unresolved);
        let parent = self.owner(specific);
        let mut scope = parent.value;
        out.merge(parent);
        if name.segments.is_empty() || name.segments.iter().any(String::is_empty) {
            out.problem(
                Completeness::Invalid,
                "KQ_REFERENCE_NAME",
                specific,
                "Reference path must have nonempty segments",
            );
            return out;
        }
        let mut scopes = vec![];
        let mut seen = BTreeSet::new();
        while let Some(id) = scope {
            if !seen.insert(id) {
                break;
            }
            scopes.push(id);
            let parent = self.owner(id);
            scope = parent.value;
            out.merge(parent);
        }
        if name.absolute {
            scopes = scopes.last().copied().into_iter().collect();
        }
        let mut found = vec![];
        for namespace in scopes {
            found = self.reference_members(namespace, &name.segments[0], false, &mut out);
            if out.completeness != Completeness::Complete {
                out.value = Resolution::Incomplete;
                return out;
            }
            if !found.is_empty() {
                break;
            }
        }
        for segment in &name.segments[1..] {
            if found.len() != 1 {
                break;
            }
            if !self.is(found[0], c::NAMESPACE) {
                found.clear();
                break;
            }
            found = self.reference_members(found[0], segment, true, &mut out);
            if out.completeness != Completeness::Complete {
                out.value = Resolution::Incomplete;
                return out;
            }
        }
        match found.as_slice() {
            [] => out.problem(
                Completeness::Invalid,
                "KQ_UNRESOLVED",
                specific,
                format!("Unresolved reference {}", name.segments.join("::")),
            ),
            [target] if self.is(*target, expected) => {
                out.value = Resolution::Resolved(*target);
                let premises: Vec<_> = out
                    .positive_dependencies
                    .iter()
                    .copied()
                    .map(Evidence::Fact)
                    .chain(
                        out.search_dependencies
                            .iter()
                            .cloned()
                            .map(Evidence::Search),
                    )
                    .collect();
                out.prove(
                    QueryKind::ResolveReference,
                    specific,
                    *target,
                    Rule::DeclaredReferenceResolution,
                    premises,
                );
            }
            [target] => {
                out.value = Resolution::WrongKind(*target);
                out.problem(
                    Completeness::Invalid,
                    "KQ_REFERENCE_KIND",
                    specific,
                    "Reference target has the wrong metaclass",
                );
            }
            _ => {
                out.value = Resolution::Ambiguous(found);
                out.problem(
                    Completeness::Invalid,
                    "KQ_AMBIGUOUS",
                    specific,
                    format!("Ambiguous reference {}", name.segments.join("::")),
                );
            }
        }
        out
    }
    fn reference_members(
        &self,
        namespace: ElementId,
        name: &str,
        public_only: bool,
        out: &mut QueryResult<Resolution>,
    ) -> Vec<ElementId> {
        if self.context().pending_namespace_scopes.contains(&namespace) {
            out.search_dependencies
                .insert(SearchDependency::NamespaceMembers { namespace });
            out.problem(
                Completeness::Incomplete,
                "KQ_PENDING_NAMESPACE",
                namespace,
                "Unavailable project syntax may contribute declarations to this namespace",
            );
        }
        if self
            .context()
            .pending_specialization_scopes
            .contains(&namespace)
        {
            out.problem(
                Completeness::Incomplete,
                "KQ_RESOLUTION_SCOPE",
                namespace,
                "Pending specialization assertions may affect inherited name resolution",
            );
        }
        if self.is(namespace, c::TYPE) {
            let specializations = self.direct_specializations(namespace);
            if !specializations.value.is_empty() {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_RESOLUTION_SCOPE",
                    namespace,
                    "Inherited name resolution is outside the declared-name slice",
                );
            }
            out.merge(specializations);
        }
        let owned = self.owned_relationships(namespace);
        for &relationship in &owned.value {
            if self.is(relationship, c::IMPORT)
                || self.is(relationship, c::SPECIALIZATION)
                || self.is(relationship, c::CONJUGATION)
            {
                out.problem(Completeness::Incomplete, "KQ_RESOLUTION_SCOPE", namespace, "Imported/inherited/conjugated name resolution is outside the declared-name slice");
            }
        }
        out.merge(owned);
        let lookup = self.lookup_declared_member(namespace, name);
        let mut targets = lookup.value.clone();
        out.merge(lookup);
        if public_only {
            let memberships = self.memberships(namespace);
            let mut visible = BTreeSet::new();
            for &membership in &memberships.value {
                self.property(out, membership, p::MEMBERSHIP_VISIBILITY);
                let view =
                    views::Membership::try_new(membership, self.model()).expect("membership query");
                let visibility = self.accept(out, membership, view.visibility());
                let registry = self.model().registry();
                let agq_kernel::metamodel::ValueKind::Enumeration(domain) = registry
                    .property(p::MEMBERSHIP_VISIBILITY)
                    .expect("descriptor")
                    .value_kind
                else {
                    unreachable!("visibility domain")
                };
                let is_public = visibility.is_some_and(|literal| {
                    registry
                        .enumeration(domain)
                        .expect("visibility enumeration")
                        .literals
                        .get(&literal)
                        .is_some_and(|n| n == "public")
                });
                let member = self.member(membership);
                if is_public {
                    visible.extend(member.value);
                }
                out.merge(member);
            }
            out.merge(memberships);
            targets.retain(|t| visible.contains(t));
        }
        targets
    }
}
