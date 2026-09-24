//! Owned projections and identity-driven diffs over one immutable request binding.
use super::*;
use agq_kerml::{classes, properties};
use agq_kerml_semantics::{Completeness, EffectiveNames, MemberAccess, MemberMatch, QueryResult};
use agq_kernel::{
    AssociationOccurrenceId, ElementId, EnumerationLiteralId, MetaclassId, ModelView, PropertyId,
    provenance::{DeclaredOrigin, Dependency, FactKey, Origin, SourceOrigin},
    value::{SlotValue, Value},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Declared, derived and navigation provenance remain distinct in projections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginDto {
    /// Original authored/imported/generated assertion.
    Declared(DeclaredOrigin),
    /// Producer rule and its immediate canonical dependencies.
    Derived {
        /// Canonical identity of the producer rule that supplied this fact.
        rule: agq_kernel::RuleId,
        /// Immediate declared or derived facts supporting this result.
        dependencies: BTreeSet<Dependency>,
    },
    /// Read-only projection of canonical association occurrences.
    AssociationOccurrences(BTreeSet<AssociationOccurrenceId>),
}
impl From<&Origin> for OriginDto {
    fn from(origin: &Origin) -> Self {
        match origin {
            Origin::Declared(value) => Self::Declared(value.clone()),
            Origin::Derived(value) => Self::Derived {
                rule: value.rule,
                dependencies: value.dependencies.clone(),
            },
            Origin::AssociationOccurrences(value) => Self::AssociationOccurrences(value.clone()),
        }
    }
}
/// Exact primitive carriers and canonical reference identities, without JSON maps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalarValueDto {
    /// Boolean primitive value.
    Boolean(bool),
    /// Arbitrary-precision integer rendered without a floating-point conversion.
    Integer(String),
    /// Exact real value in the kernel's textual representation.
    Real(String),
    /// Unmodified string primitive value.
    String(String),
    /// Canonical enumeration literal with an optional descriptor display name.
    Enumeration {
        /// Stable identity of the enumeration literal.
        id: EnumerationLiteralId,
        /// Name found in the property's declared enumeration domain, when available.
        name: Option<String>,
    },
    /// Reference to an original canonical element identity.
    Reference(ElementId),
}
/// Collection shape/order is semantic information, including singleton collections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotValueDto {
    /// Single primitive or reference value, distinct from a singleton collection.
    Scalar(ScalarValueDto),
    /// Collection preserving its canonical semantic order.
    Ordered(Vec<ScalarValueDto>),
    /// Unordered unique values; serialization order carries no ordering semantics.
    Set(Vec<ScalarValueDto>),
    /// Unordered values retaining duplicate occurrences.
    Bag(Vec<ScalarValueDto>),
}
/// One present canonical property; absent properties are not synthesized defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyDto {
    /// Canonical metamodel property identity.
    pub id: PropertyId,
    /// Property name from the bound metamodel descriptor.
    pub name: String,
    /// Present value with its scalar or collection shape preserved.
    pub value: SlotValueDto,
    /// Declared, derived or association provenance of this property value.
    pub origin: OriginDto,
}
/// Current graph record projection, explicitly separate from an effective query.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementDto {
    /// Original canonical element identity, independent of names and source paths.
    pub id: ElementId,
    /// Immutable project revision from which this projection was read.
    pub revision_id: ProjectRevisionId,
    /// Canonical metaclass descriptor identity.
    pub metaclass: MetaclassId,
    /// Metaclass name from the bound descriptor registry.
    pub metaclass_name: String,
    /// Declared name path through ownership, when every required name is available.
    pub declared_qualified_name: Option<String>,
    /// Present canonical slots with their value shape and provenance.
    pub properties: Vec<PropertyDto>,
    /// Authored document, source revision and byte range, when mapped to this element.
    pub source: Option<SourceOrigin>,
    /// Provenance of the element record itself.
    pub origin: OriginDto,
}
/// Effective answers retain the language contract's owned evidence, diagnostics,
/// completeness and semantic context without leaking a borrowed evaluator.
#[derive(Clone, Debug)]
pub struct EffectiveAnswer<T> {
    /// Immutable project revision that binds the query and all its evidence.
    pub revision_id: ProjectRevisionId,
    /// Language query value, completeness, diagnostics, dependencies and evidence.
    pub answer: QueryResult<T>,
}
/// A source location can intentionally map to several canonical semantic records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceMatches {
    /// No element origin contains the supplied document revision and byte offset.
    Missing,
    /// Exactly one canonical element origin contains the supplied position.
    Unique(ElementId),
    /// All matching canonical identities when source ranges overlap.
    Ambiguous(Vec<ElementId>),
}
impl BoundRevision {
    fn model(&self) -> Result<&ModelView, ServiceError> {
        self.working
            .semantic_model()
            .ok_or_else(|| ServiceError::Invalid("current graph unavailable".into()))
    }
    /// Current authored graph records, excluding immutable accepted dependency records.
    /// Derived records are included with their explicit producer provenance.
    pub fn current_elements(&self) -> Result<Vec<ElementDto>, ServiceError> {
        let model = self.model()?;
        model
            .elements()
            .filter(|record| !self.is_standard(record.id()))
            .map(|record| self.project_element(model, record.id()))
            .collect()
    }
    /// Current graph lookup by canonical identity. Standards may be queried explicitly.
    pub fn current_element(&self, id: ElementId) -> Result<ElementDto, ServiceError> {
        self.project_element(self.model()?, id)
    }
    /// Original declared graph without presenting producer additions as authored facts.
    pub fn declared_element(&self, id: ElementId) -> Result<ElementDto, ServiceError> {
        let model = self
            .working
            .strict_snapshot()
            .map(|snapshot| snapshot.model())
            .or_else(|| self.working.construction().map(|view| view.model()))
            .ok_or_else(|| ServiceError::Invalid("declared graph unavailable".into()))?;
        self.project_element(model, id)
    }
    /// Authored top-level owned members below the private workspace root.
    pub fn roots(&self) -> Result<Vec<ElementDto>, ServiceError> {
        let q = self
            .working
            .kerml_queries()
            .map_err(|e| ServiceError::Invalid(format!("{e:?}")))?;
        let answer = q.memberships(self.working.root());
        if answer.completeness != Completeness::Complete {
            return Err(ServiceError::Invalid(
                "root population is not Complete".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for membership in answer.value {
            let member = q.member(membership);
            if member.completeness != Completeness::Complete {
                return Err(ServiceError::Invalid(
                    "root membership endpoint is not Complete".into(),
                ));
            }
            if let Some(id) = member.value {
                ids.insert(id);
            }
        }
        ids.into_iter()
            .filter(|id| !self.is_standard(*id))
            .map(|id| self.current_element(id))
            .collect()
    }
    /// Canonical Relationship records incident to this element in the current graph.
    pub fn relationships(&self, id: ElementId) -> Result<Vec<ElementDto>, ServiceError> {
        self.current_element(id)?;
        let model = self.model()?;
        let mut ids = BTreeSet::new();
        for reference in model.incoming(id) {
            if let Some(record) = model.element(reference.source)
                && model
                    .registry()
                    .is_subtype(record.metaclass(), classes::RELATIONSHIP)
                    .unwrap_or(false)
            {
                ids.insert(record.id());
            }
        }
        for reference in model.outgoing(id) {
            if let Some(record) = model.element(reference.target)
                && model
                    .registry()
                    .is_subtype(record.metaclass(), classes::RELATIONSHIP)
                    .unwrap_or(false)
            {
                ids.insert(record.id());
            }
        }
        ids.into_iter().map(|id| self.current_element(id)).collect()
    }
    /// Producer-closed membership query with original Membership and element identities.
    pub fn effective_members(
        &self,
        namespace: ElementId,
    ) -> Result<EffectiveAnswer<Vec<MemberMatch>>, ServiceError> {
        let q = self
            .working
            .kerml_queries()
            .map_err(|e| ServiceError::Invalid(format!("{e:?}")))?;
        Ok(EffectiveAnswer {
            revision_id: self.working.revision(),
            answer: q.namespace_members(namespace, MemberAccess::All),
        })
    }
    /// Effective naming retains ambiguity, negative evidence and closure diagnostics.
    pub fn effective_names(
        &self,
        element: ElementId,
    ) -> Result<EffectiveAnswer<EffectiveNames>, ServiceError> {
        let q = self
            .working
            .kerml_queries()
            .map_err(|e| ServiceError::Invalid(format!("{e:?}")))?;
        Ok(EffectiveAnswer {
            revision_id: self.working.revision(),
            answer: q.effective_names(element),
        })
    }
    /// Exact source revision and half-open byte range for one canonical element.
    pub fn source_origin(&self, element: ElementId) -> Option<SourceOrigin> {
        self.working
            .source_for_fact(FactKey::Element(element))
            .cloned()
    }
    /// Return every semantic element containing this exact source position.
    pub fn elements_at_source(
        &self,
        document: DocumentId,
        source: SourceRevisionId,
        offset: u64,
    ) -> Result<SourceMatches, ServiceError> {
        let mut matches = Vec::new();
        for element in self.model()?.elements() {
            if let Some(origin) = self.source_origin(element.id())
                && origin.document == document
                && origin.revision == source
                && origin.range.start() <= offset
                && offset < origin.range.end()
            {
                matches.push(element.id());
            }
        }
        Ok(match matches.as_slice() {
            [] => SourceMatches::Missing,
            [one] => SourceMatches::Unique(*one),
            _ => SourceMatches::Ambiguous(matches),
        })
    }
    /// Source/frontend diagnostics are retained as typed owned values.
    pub fn diagnostics(&self) -> Vec<agq_kerml_text::SourceDiagnostic> {
        self.working.diagnostics().to_vec()
    }
    fn is_standard(&self, id: ElementId) -> bool {
        self.working
            .accepted_sysml()
            .overlay()
            .model()
            .element(id)
            .is_some()
    }
    fn project_element(
        &self,
        model: &ModelView,
        id: ElementId,
    ) -> Result<ElementDto, ServiceError> {
        let record = model
            .element(id)
            .ok_or_else(|| RepositoryError::NotFound(format!("element {id}")))?;
        let metaclass = model
            .registry()
            .class(record.metaclass())
            .map_err(|e| ServiceError::Invalid(e.to_string()))?;
        let mut projected = Vec::new();
        for (id, slot) in record.slots() {
            let property = model
                .registry()
                .property(id)
                .map_err(|e| ServiceError::Invalid(e.to_string()))?;
            let scalar = |value: &Value| -> ScalarValueDto {
                match value {
                    Value::Boolean(v) => ScalarValueDto::Boolean(*v),
                    Value::Integer(v) => ScalarValueDto::Integer(v.to_string()),
                    Value::Real(v) => ScalarValueDto::Real(v.to_string()),
                    Value::String(v) => ScalarValueDto::String(v.clone()),
                    Value::Reference(v) => ScalarValueDto::Reference(*v),
                    Value::Enumeration(id) => {
                        let name = if let agq_kernel::metamodel::ValueKind::Enumeration(domain) =
                            property.value_kind
                        {
                            model
                                .registry()
                                .enumeration(domain)
                                .ok()
                                .and_then(|domain| domain.literals.get(id).cloned())
                        } else {
                            None
                        };
                        ScalarValueDto::Enumeration { id: *id, name }
                    }
                }
            };
            let value = match slot.value() {
                SlotValue::Scalar(v) => SlotValueDto::Scalar(scalar(v)),
                SlotValue::Ordered(v) => SlotValueDto::Ordered(v.iter().map(scalar).collect()),
                SlotValue::Set(v) => SlotValueDto::Set(v.iter().map(scalar).collect()),
                SlotValue::Bag(v) => SlotValueDto::Bag(v.iter().map(scalar).collect()),
            };
            projected.push(PropertyDto {
                id,
                name: property.name.clone(),
                value,
                origin: slot.origin().into(),
            });
        }
        Ok(ElementDto {
            id,
            revision_id: self.working.revision(),
            metaclass: record.metaclass(),
            metaclass_name: metaclass.name.clone(),
            declared_qualified_name: declared_qualified_name(model, id, self.working.root()),
            properties: projected,
            source: self.source_origin(id),
            origin: record.origin().into(),
        })
    }
}
fn declared_qualified_name(
    model: &ModelView,
    mut id: ElementId,
    root: ElementId,
) -> Option<String> {
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    while id != root {
        if !seen.insert(id) {
            return None;
        }
        let record = model.element(id)?;
        let slot = record.slot(properties::ELEMENT_DECLARED_NAME)?;
        let SlotValue::Scalar(Value::String(name)) = slot.value() else {
            return None;
        };
        names.push(name.clone());
        let relationship = model
            .navigation_slot(id, properties::ELEMENT_OWNING_RELATIONSHIP)
            .and_then(|slot| {
                slot.value().values().find_map(|value| {
                    if let Value::Reference(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
            });
        let owner = relationship
            .and_then(|relationship| {
                model.navigation_slot(
                    relationship,
                    properties::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                )
            })
            .and_then(|slot| {
                slot.value().values().find_map(|value| {
                    if let Value::Reference(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
            });
        match owner {
            Some(owner) => id = owner,
            None => break,
        }
    }
    names.reverse();
    (!names.is_empty()).then(|| names.join("::"))
}

/// Canonical relationship identities include element records and association occurrences.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RelationshipIdentity {
    /// Canonical element record whose metaclass is a Relationship subtype.
    Element(ElementId),
    /// Canonical association occurrence, retaining its independent occurrence identity.
    Occurrence(AssociationOccurrenceId),
}
/// Language-neutral identity sets preserve declared versus effective consequences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementChanges {
    /// Identities present only in the later revision for this fact category.
    pub added: Vec<ElementId>,
    /// Identities present only in the earlier revision for this fact category.
    pub removed: Vec<ElementId>,
    /// Retained identities whose declared or derived facts changed in this category.
    pub changed: Vec<ElementId>,
}
/// Exact document change, with the smallest enclosing UTF-8 byte replacement ranges.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentChange {
    /// Stable document identity used to match both revisions independently of path.
    pub document_id: DocumentId,
    /// Earlier source binding; absent when the document was added.
    pub before: Option<DocumentManifest>,
    /// Later source binding; absent when the document was removed.
    pub after: Option<DocumentManifest>,
    /// Half-open replacement byte range in the earlier UTF-8 source, when present.
    pub before_range: Option<(usize, usize)>,
    /// Half-open replacement byte range in the later UTF-8 source, when present.
    pub after_range: Option<(usize, usize)>,
}
/// Diff is a projection over canonical identities, never an alternate semantic graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDiff {
    /// Exact earlier revision in the comparison.
    pub from: ProjectRevisionId,
    /// Exact later revision in the comparison.
    pub to: ProjectRevisionId,
    /// Added, removed or changed document bindings and available source ranges.
    pub documents: Vec<DocumentChange>,
    /// Changes to declared records and declared slots, excluding derived-only changes.
    pub declared: ElementChanges,
    /// Changes to producer-derived records or facts, including slots on declared records.
    pub derived: ElementChanges,
    /// Relationship or association occurrence identities added by the later revision.
    pub relationships_added: Vec<RelationshipIdentity>,
    /// Relationship or association occurrence identities removed by the later revision.
    pub relationships_removed: Vec<RelationshipIdentity>,
    /// Retained relationship or association occurrence identities whose records changed.
    pub relationships_changed: Vec<RelationshipIdentity>,
    /// Whether the persisted Working or Validated state variant changed.
    pub validation_changed: bool,
}
pub(crate) fn revision_diff(
    before: &BoundRevision,
    after: &BoundRevision,
) -> Result<RevisionDiff, ServiceError> {
    let mut diff = RevisionDiff {
        from: before.working.revision(),
        to: after.working.revision(),
        documents: vec![],
        declared: ElementChanges::default(),
        derived: ElementChanges::default(),
        relationships_added: vec![],
        relationships_removed: vec![],
        relationships_changed: vec![],
        validation_changed: std::mem::discriminant(&before.manifest.validation)
            != std::mem::discriminant(&after.manifest.validation),
    };
    let left: BTreeMap<_, _> = before
        .manifest
        .documents
        .iter()
        .map(|d| (d.document_id, d))
        .collect();
    let right: BTreeMap<_, _> = after
        .manifest
        .documents
        .iter()
        .map(|d| (d.document_id, d))
        .collect();
    for id in left
        .keys()
        .chain(right.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        if left.get(&id) == right.get(&id) {
            continue;
        }
        let (before_range, after_range) =
            match (before.working.document(id), after.working.document(id)) {
                (Some(a), Some(b)) => {
                    let (a, b) = changed_ranges(a.source(), b.source());
                    (Some(a), Some(b))
                }
                (Some(a), None) => (Some((0, a.source().len())), None),
                (None, Some(b)) => (None, Some((0, b.source().len()))),
                _ => (None, None),
            };
        diff.documents.push(DocumentChange {
            document_id: id,
            before: left.get(&id).map(|d| (*d).clone()),
            after: right.get(&id).map(|d| (*d).clone()),
            before_range,
            after_range,
        });
    }
    let left = before.model()?;
    let right = after.model()?;
    let ids: BTreeSet<_> = left
        .elements()
        .chain(right.elements())
        .map(|r| r.id())
        .filter(|id| !before.is_standard(*id))
        .collect();
    for id in ids {
        let a = left.element(id);
        let b = right.element(id);
        if a == b {
            continue;
        }
        let record = b.or(a).expect("union element");
        let changes = if matches!(record.origin(), Origin::Declared(_)) {
            &mut diff.declared
        } else {
            &mut diff.derived
        };
        let declared_changed = match (a, b) {
            (Some(a), Some(b)) if matches!(record.origin(), Origin::Declared(_)) => {
                a.metaclass() != b.metaclass()
                    || a.origin() != b.origin()
                    || a.slots()
                        .chain(b.slots())
                        .map(|(p, _)| p)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .any(|p| left.declared_slot(id, p) != right.declared_slot(id, p))
            }
            _ => true,
        };
        match (a, b) {
            (None, Some(_)) => changes.added.push(id),
            (Some(_), None) => changes.removed.push(id),
            _ if declared_changed => changes.changed.push(id),
            _ => {}
        }
        // Derived slot changes on an existing declared record remain consequences.
        if let (Some(a), Some(b)) = (a, b) {
            let derived_changed = a
                .slots()
                .chain(b.slots())
                .map(|(p, _)| p)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .any(|p| {
                    a.slot(p) != b.slot(p)
                        && a.slot(p)
                            .into_iter()
                            .chain(b.slot(p))
                            .any(|s| !matches!(s.origin(), Origin::Declared(_)))
                });
            if derived_changed && !diff.derived.changed.contains(&id) {
                diff.derived.changed.push(id);
            }
        }
        if right
            .registry()
            .is_subtype(record.metaclass(), classes::RELATIONSHIP)
            .unwrap_or(false)
        {
            let relation = RelationshipIdentity::Element(id);
            match (a, b) {
                (None, Some(_)) => diff.relationships_added.push(relation),
                (Some(_), None) => diff.relationships_removed.push(relation),
                _ => diff.relationships_changed.push(relation),
            }
        }
    }
    let occurrences: BTreeSet<_> = left
        .association_occurrences()
        .chain(right.association_occurrences())
        .map(|r| r.id())
        .collect();
    for id in occurrences {
        let a = left.association_occurrence(id);
        let b = right.association_occurrence(id);
        if a == b {
            continue;
        }
        let id = RelationshipIdentity::Occurrence(id);
        match (a, b) {
            (None, Some(_)) => diff.relationships_added.push(id),
            (Some(_), None) => diff.relationships_removed.push(id),
            _ => diff.relationships_changed.push(id),
        }
    }
    Ok(diff)
}
fn changed_ranges(a: &str, b: &str) -> ((usize, usize), (usize, usize)) {
    let mut start = a.bytes().zip(b.bytes()).take_while(|(a, b)| a == b).count();
    while !a.is_char_boundary(start) || !b.is_char_boundary(start) {
        start -= 1;
    }
    let mut suffix = a[start..]
        .bytes()
        .rev()
        .zip(b[start..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !a.is_char_boundary(a.len() - suffix) || !b.is_char_boundary(b.len() - suffix) {
        suffix -= 1;
    }
    ((start, a.len() - suffix), (start, b.len() - suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_range_respects_unicode_and_insertions() {
        assert_eq!(changed_ranges("AöB", "AåB"), ((1, 3), (1, 3)));
        assert_eq!(changed_ranges("ab", "axb"), ((1, 1), (1, 2)));
        assert_eq!(changed_ranges("ab", "ab"), ((2, 2), (2, 2)));
    }
}
