//! Evidence-bearing KerML semantic queries over one immutable kernel snapshot.
//!
//! This crate deliberately has no parser, mutable expansion pass, or cache as
//! canonical state.  Its result contract records both facts read and searched
//! spaces whose absence contributed to an answer.
#![forbid(unsafe_code)]

use agq_kerml::{classes, properties};
use agq_kernel::provenance::FactKey;
use agq_kernel::{ElementId, ModelView, PropertyId, RevisionId, Snapshot};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Versioned inputs whose changes can alter an answer even for the same revision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticContext {
    pub revision: RevisionId,
    pub metamodel_version: String,
    pub rule_set_version: String,
    pub pinned_libraries: BTreeSet<String>,
    pub options: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_kernel::{
        ElementId, RuleId, Snapshot,
        derived::{DerivationBuilder, DerivedOverlay},
        metamodel::ValueKind,
        provenance::{DeclaredOrigin, Dependency, Explanation as KernelExplanation},
        value::{SlotValue, Value},
    };
    use std::{collections::BTreeSet, sync::Arc};

    const A: ElementId = ElementId::from_u128(1);
    const B: ElementId = ElementId::from_u128(2);
    const C: ElementId = ElementId::from_u128(3);
    const D: ElementId = ElementId::from_u128(4);
    const FA: ElementId = ElementId::from_u128(11);
    const FB: ElementId = ElementId::from_u128(12);
    const FC: ElementId = ElementId::from_u128(13);
    const FD: ElementId = ElementId::from_u128(14);
    fn origin() -> DeclaredOrigin {
        DeclaredOrigin::Authored { source: None }
    }
    fn reference(id: ElementId) -> SlotValue {
        SlotValue::Scalar(Value::Reference(id))
    }
    fn fixture(with_cycle: bool) -> DerivedOverlay {
        let registry = Arc::new(agq_kerml::registry().unwrap());
        let empty = Snapshot::new(registry.clone());
        let mut c = empty.change_set();
        let mut ids = vec![
            (A, classes::TYPE),
            (B, classes::TYPE),
            (C, classes::TYPE),
            (D, classes::TYPE),
            (FA, classes::FEATURE),
            (FB, classes::FEATURE),
            (FC, classes::FEATURE),
            (FD, classes::FEATURE),
        ];
        ids.extend(
            (20..if with_cycle { 25 } else { 24 })
                .map(|n| (ElementId::from_u128(n), classes::SPECIALIZATION)),
        );
        ids.extend((30..34).map(|n| (ElementId::from_u128(n), classes::FEATURE_MEMBERSHIP)));
        ids.push((ElementId::from_u128(40), classes::FEATURE_TYPING));
        ids.push((ElementId::from_u128(41), classes::SUBSETTING));
        ids.push((ElementId::from_u128(42), classes::REDEFINITION));
        for (id, class) in ids {
            c.create(id, class, origin());
            for p in registry
                .effective_properties(class)
                .unwrap()
                .filter(|p| !p.derived && p.multiplicity.lower > 0)
            {
                let value = match p.value_kind {
                    ValueKind::Boolean => Value::Boolean(false),
                    ValueKind::String => Value::String(format!("x{id}")),
                    ValueKind::Enumeration(domain) => Value::Enumeration(
                        *registry
                            .enumeration(domain)
                            .unwrap()
                            .literals
                            .keys()
                            .next()
                            .unwrap(),
                    ),
                    ValueKind::Reference(_) => continue,
                    _ => continue,
                };
                c.set(id, p.id, SlotValue::Scalar(value), origin());
            }
        }
        for (r, specific, general) in [(20, B, A), (21, C, A), (22, D, B), (23, D, C)] {
            c.set(
                ElementId::from_u128(r),
                properties::SPECIALIZATION_SPECIFIC,
                reference(specific),
                origin(),
            )
            .set(
                ElementId::from_u128(r),
                properties::SPECIALIZATION_GENERAL,
                reference(general),
                origin(),
            );
        }
        if with_cycle {
            c.set(
                ElementId::from_u128(24),
                properties::SPECIALIZATION_SPECIFIC,
                reference(A),
                origin(),
            )
            .set(
                ElementId::from_u128(24),
                properties::SPECIALIZATION_GENERAL,
                reference(D),
                origin(),
            );
        }
        c.set(
            ElementId::from_u128(40),
            properties::FEATURE_TYPING_TYPED_FEATURE,
            reference(FC),
            origin(),
        )
        .set(
            ElementId::from_u128(40),
            properties::FEATURE_TYPING_TYPE,
            reference(A),
            origin(),
        )
        .set(
            ElementId::from_u128(41),
            properties::SUBSETTING_SUBSETTING_FEATURE,
            reference(FC),
            origin(),
        )
        .set(
            ElementId::from_u128(41),
            properties::SUBSETTING_SUBSETTED_FEATURE,
            reference(FA),
            origin(),
        )
        .set(
            ElementId::from_u128(42),
            properties::REDEFINITION_REDEFINING_FEATURE,
            reference(FD),
            origin(),
        )
        .set(
            ElementId::from_u128(42),
            properties::REDEFINITION_REDEFINED_FEATURE,
            reference(FA),
            origin(),
        );
        let snapshot = empty.apply(&c).unwrap();
        let mut derived = DerivationBuilder::new(snapshot);
        for (r, ty, feature) in [(30, A, FA), (31, B, FB), (32, D, FC), (33, D, FD)] {
            let id = ElementId::from_u128(r);
            let evidence = || KernelExplanation {
                rule: RuleId::from_u128(9),
                dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id))]),
            };
            derived.property(
                id,
                properties::FEATURE_MEMBERSHIP_OWNING_TYPE,
                reference(ty),
                evidence(),
            );
            derived.property(
                id,
                properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE,
                reference(feature),
                evidence(),
            );
        }
        derived.build().unwrap()
    }
    fn queries(overlay: &DerivedOverlay) -> KerMlQueries<'_> {
        KerMlQueries::for_model(
            overlay.model(),
            SemanticContext {
                revision: overlay.base_revision(),
                metamodel_version: "KerML 1.0".into(),
                rule_set_version: "kerml-query/1".into(),
                pinned_libraries: BTreeSet::from(["kernel-semantic-library@1.0.0".into()]),
                options: BTreeMap::new(),
            },
        )
    }
    #[test]
    fn chains_diamonds_and_explanations_are_deterministic() {
        let s = fixture(false);
        let q = queries(&s);
        let result = q.all_specializations(D);
        assert_eq!(
            result.value,
            vec![A, B, C],
            "direct D={:?}",
            q.direct_specializations(D)
        );
        assert_eq!(result.completeness, Completeness::Complete);
        assert!(
            result
                .explanations
                .iter()
                .any(|e| e.relationship == Some(ElementId::from_u128(22)))
        );
        assert!(
            result
                .search_dependencies
                .contains(&SearchDependency::Instances {
                    class: classes::SPECIALIZATION
                })
        );
        assert_eq!(q.all_specializations(D).value, result.value);
    }
    #[test]
    fn effective_features_inherit_without_copying_and_obey_redefinition() {
        let s = fixture(false);
        let q = queries(&s);
        assert_eq!(q.feature_types(FC).value, vec![A]);
        assert_eq!(q.subsetted_features(FC).value, vec![FA]);
        assert_eq!(q.redefined_features(FD).value, vec![FA]);
        let effective = q.effective_features(D);
        assert_eq!(effective.value, vec![FB, FC, FD]);
        assert!(
            effective
                .explanations
                .iter()
                .any(|e| e.rule == "KerML inherited effective feature")
        );
        assert!(s.model().element(FA).is_some());
        assert_eq!(q.direct_features(D).value, vec![FC, FD]);
    }
    #[test]
    fn absence_has_a_search_space_dependency() {
        let s = fixture(false);
        let r = queries(&s).direct_features(C);
        assert!(r.value.is_empty());
        assert_eq!(r.completeness, Completeness::Complete);
        assert!(
            r.search_dependencies
                .contains(&SearchDependency::Instances {
                    class: classes::FEATURE_MEMBERSHIP
                })
        );
    }
    #[test]
    fn illegal_specialization_cycle_is_incomplete() {
        let s = fixture(true);
        let result = queries(&s).all_specializations(D);
        assert_eq!(result.completeness, Completeness::Incomplete);
        assert!(result.diagnostics.iter().any(|d| d.code == "KQ003"));
    }
}

/// Stable, comparable identity of a semantic context; it is deliberately not a revision alone.
pub type SemanticContextId = SemanticContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completeness {
    Complete,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchDependency {
    /// An answer depends on the complete set of instances of this metaclass.
    Instances { class: agq_kernel::MetaclassId },
    /// An answer depends on all values of a property on one subject.
    PropertySet {
        element: ElementId,
        property: PropertyId,
    },
    /// An answer depends on the membership population of a namespace.
    NamespaceMembers { namespace: ElementId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub subject: ElementId,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub rule: &'static str,
    pub conclusion: ElementId,
    pub relationship: Option<ElementId>,
    pub evidence: Vec<ElementId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResult<T> {
    pub value: T,
    pub completeness: Completeness,
    pub diagnostics: Vec<Diagnostic>,
    pub positive_dependencies: BTreeSet<FactKey>,
    pub search_dependencies: BTreeSet<SearchDependency>,
    pub explanations: Vec<Explanation>,
}

impl<T> QueryResult<T> {
    fn complete(value: T) -> Self {
        Self {
            value,
            completeness: Completeness::Complete,
            diagnostics: vec![],
            positive_dependencies: BTreeSet::new(),
            search_dependencies: BTreeSet::new(),
            explanations: vec![],
        }
    }
}

/// Stateless evaluator. Dropping it cannot affect semantics; memoization can wrap this API later.
pub struct KerMlQueries<'m> {
    model: &'m ModelView,
    context: SemanticContext,
}

impl<'m> KerMlQueries<'m> {
    /// Evaluate an immutable declared snapshot or derived overlay model with an
    /// explicitly supplied context. The caller owns the revision binding.
    pub fn for_model(model: &'m ModelView, context: SemanticContext) -> Self {
        Self { model, context }
    }
    pub fn for_snapshot(snapshot: &'m Snapshot, mut context: SemanticContext) -> Self {
        context.revision = snapshot.revision();
        Self {
            model: snapshot.model(),
            context,
        }
    }
    pub fn context(&self) -> &SemanticContext {
        &self.context
    }

    /// The direct owner derived from an owning membership. No inferred owner is fabricated.
    pub fn owner(&self, element: ElementId) -> QueryResult<Option<ElementId>> {
        let mut r = QueryResult::complete(None);
        r.search_dependencies.insert(SearchDependency::Instances {
            class: classes::OWNING_MEMBERSHIP,
        });
        for membership in self.instances(classes::OWNING_MEMBERSHIP) {
            if self.reference(
                membership,
                properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT,
            ) == Some(element)
            {
                r.positive_dependencies.insert(FactKey::Element(membership));
                r.positive_dependencies.insert(FactKey::Property {
                    element: membership,
                    property: properties::OWNING_MEMBERSHIP_OWNED_MEMBER_ELEMENT,
                });
                // A membership's enclosing namespace is the owner when its endpoint is stored.
                if let Some(owner) = self.reference(
                    membership,
                    properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE,
                ) {
                    r.value = Some(owner);
                    r.positive_dependencies.insert(FactKey::Property {
                        element: membership,
                        property: properties::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE,
                    });
                    r.explanations.push(Explanation {
                        rule: "KerML Element::owner via OwningMembership",
                        conclusion: element,
                        relationship: Some(membership),
                        evidence: vec![membership, owner],
                    });
                } else {
                    r.completeness = Completeness::Incomplete;
                    r.diagnostics.push(Diagnostic {
                        code: "KQ001",
                        subject: membership,
                        message: "owning membership has no stored owning namespace".into(),
                    });
                }
                break;
            }
        }
        r
    }

    pub fn direct_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut r = QueryResult::complete(vec![]);
        r.search_dependencies.insert(SearchDependency::Instances {
            class: classes::FEATURE_MEMBERSHIP,
        });
        for relationship in self.instances(classes::FEATURE_MEMBERSHIP) {
            if self.reference(relationship, properties::FEATURE_MEMBERSHIP_OWNING_TYPE) == Some(ty)
            {
                match self.reference(
                    relationship,
                    properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE,
                ) {
                    Some(feature) => {
                        r.value.push(feature);
                        r.positive_dependencies.extend([
                            FactKey::Element(relationship),
                            FactKey::Property {
                                element: relationship,
                                property: properties::FEATURE_MEMBERSHIP_OWNING_TYPE,
                            },
                            FactKey::Property {
                                element: relationship,
                                property: properties::FEATURE_MEMBERSHIP_OWNED_MEMBER_FEATURE,
                            },
                        ]);
                        r.explanations.push(Explanation {
                            rule: "KerML Type::featureMembership",
                            conclusion: feature,
                            relationship: Some(relationship),
                            evidence: vec![ty, relationship],
                        });
                    }
                    None => self.incomplete(
                        &mut r,
                        "KQ002",
                        relationship,
                        "feature membership has no stored owned member feature",
                    ),
                }
            }
        }
        r.value.sort();
        r.value.dedup();
        r
    }

    pub fn direct_specializations(&self, specific: ElementId) -> QueryResult<Vec<ElementId>> {
        self.relationship_targets(
            specific,
            classes::SPECIALIZATION,
            properties::SPECIALIZATION_SPECIFIC,
            properties::SPECIALIZATION_GENERAL,
            "KerML Specialization::general",
        )
    }
    /// All general types, breadth-first and deterministic; a cycle is incomplete/invalid evidence.
    pub fn all_specializations(&self, specific: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = QueryResult::complete(vec![]);
        let mut queue = VecDeque::from([specific]);
        let mut seen = BTreeSet::from([specific]);
        while let Some(current) = queue.pop_front() {
            let direct = self.direct_specializations(current);
            let targets = direct.value.clone();
            self.merge(&mut out, direct);
            for general in targets {
                if seen.insert(general) {
                    out.value.push(general);
                    queue.push_back(general);
                } else if general == specific {
                    self.incomplete(
                        &mut out,
                        "KQ003",
                        general,
                        "specialization cycle encountered",
                    );
                }
            }
        }
        out.value.retain(|id| *id != specific);
        out.value.sort();
        out.value.dedup();
        out
    }
    pub fn feature_types(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.relationship_targets(
            feature,
            classes::FEATURE_TYPING,
            properties::FEATURE_TYPING_TYPED_FEATURE,
            properties::FEATURE_TYPING_TYPE,
            "KerML FeatureTyping::type",
        )
    }
    pub fn subsetted_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.relationship_targets(
            feature,
            classes::SUBSETTING,
            properties::SUBSETTING_SUBSETTING_FEATURE,
            properties::SUBSETTING_SUBSETTED_FEATURE,
            "KerML Subsetting::subsettedFeature",
        )
    }
    pub fn redefined_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.relationship_targets(
            feature,
            classes::REDEFINITION,
            properties::REDEFINITION_REDEFINING_FEATURE,
            properties::REDEFINITION_REDEFINED_FEATURE,
            "KerML Redefinition::redefinedFeature",
        )
    }
    /// Direct features plus inherited features. Redefined inherited features are suppressed, never copied.
    pub fn effective_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.direct_features(ty);
        let direct = out.value.clone();
        let ancestors = self.all_specializations(ty);
        self.merge(&mut out, ancestors.clone());
        let mut suppressed = BTreeSet::new();
        for feature in &direct {
            suppressed.extend(self.redefined_features(*feature).value);
        }
        for ancestor in ancestors.value {
            let inherited = self.direct_features(ancestor);
            self.merge(&mut out, inherited.clone());
            for feature in inherited.value {
                if !suppressed.contains(&feature) {
                    out.value.push(feature);
                    out.explanations.push(Explanation {
                        rule: "KerML inherited effective feature",
                        conclusion: feature,
                        relationship: None,
                        evidence: vec![ty, ancestor, feature],
                    });
                }
            }
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    fn relationship_targets(
        &self,
        source: ElementId,
        class: agq_kernel::MetaclassId,
        source_property: PropertyId,
        target_property: PropertyId,
        rule: &'static str,
    ) -> QueryResult<Vec<ElementId>> {
        let mut r = QueryResult::complete(vec![]);
        r.search_dependencies
            .insert(SearchDependency::Instances { class });
        for relationship in self.instances(class) {
            if self.reference(relationship, source_property) == Some(source) {
                match self.reference(relationship, target_property) {
                    Some(target) => {
                        r.value.push(target);
                        r.positive_dependencies.extend([
                            FactKey::Element(relationship),
                            FactKey::Property {
                                element: relationship,
                                property: source_property,
                            },
                            FactKey::Property {
                                element: relationship,
                                property: target_property,
                            },
                        ]);
                        r.explanations.push(Explanation {
                            rule,
                            conclusion: target,
                            relationship: Some(relationship),
                            evidence: vec![source, relationship, target],
                        });
                    }
                    None => self.incomplete(
                        &mut r,
                        "KQ004",
                        relationship,
                        "relationship has no stored target endpoint",
                    ),
                }
            }
        }
        r.value.sort();
        r.value.dedup();
        r
    }
    fn instances(&self, class: agq_kernel::MetaclassId) -> impl Iterator<Item = ElementId> + '_ {
        self.model
            .instances(class, true)
            .expect("generated KerML class must be registered")
            .map(|e| e.id())
    }
    fn reference(&self, element: ElementId, property: PropertyId) -> Option<ElementId> {
        self.model
            .element(element)?
            .slot(property)?
            .value()
            .values()
            .next()
            .and_then(|v| {
                if let agq_kernel::value::Value::Reference(id) = v {
                    Some(*id)
                } else {
                    None
                }
            })
    }
    fn incomplete<T>(
        &self,
        r: &mut QueryResult<T>,
        code: &'static str,
        subject: ElementId,
        message: &str,
    ) {
        r.completeness = Completeness::Incomplete;
        r.diagnostics.push(Diagnostic {
            code,
            subject,
            message: message.into(),
        });
    }
    fn merge<T>(&self, into: &mut QueryResult<Vec<T>>, mut other: QueryResult<Vec<T>>) {
        if other.completeness == Completeness::Incomplete {
            into.completeness = Completeness::Incomplete;
        }
        into.positive_dependencies
            .append(&mut other.positive_dependencies);
        into.search_dependencies
            .append(&mut other.search_dependencies);
        into.diagnostics.append(&mut other.diagnostics);
        into.explanations.append(&mut other.explanations);
    }
}
