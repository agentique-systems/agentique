use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{
    Completeness, KerMlQueries, QualifiedName, QueryResult, Resolution, SemanticContext,
    SemanticOptions,
};
use agq_kerml_syntax::{
    Declaration, DeclarationKind, DeclarationSyntax, ReferenceKind, SyntaxDocument, SyntaxNode,
    SyntaxStatus,
};
use agq_kernel::{provenance::*, value::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct ReferenceAssertion {
    pub relationship: ElementId,
    pub specific: ElementId,
    pub kind: ReferenceKind,
    pub name: QualifiedName,
    pub origin: SourceOrigin,
    pub resolution: QueryResult<Resolution>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendDiagnostic {
    pub code: &'static str,
    pub origin: SourceOrigin,
    pub message: String,
}
#[derive(Clone, Debug)]
struct Ids {
    element: ElementId,
    membership: ElementId,
}
#[derive(Debug)]
pub struct WorkingModel {
    syntax: SyntaxDocument,
    snapshot: Snapshot,
    root: ElementId,
    identities: BTreeMap<SyntaxNodeId, Ids>,
    references: Vec<ReferenceAssertion>,
    diagnostics: Vec<FrontendDiagnostic>,
}
impl WorkingModel {
    pub fn syntax(&self) -> &SyntaxDocument {
        &self.syntax
    }
    /// Structurally valid canonical facts. Pending references are separate;
    /// inspect diagnostics/validate_slice before treating the document as valid.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn root(&self) -> ElementId {
        self.root
    }
    pub fn element_for(&self, node: SyntaxNodeId) -> Option<ElementId> {
        self.identities.get(&node).map(|ids| ids.element)
    }
    pub fn references(&self) -> &[ReferenceAssertion] {
        &self.references
    }
    pub fn diagnostics(&self) -> &[FrontendDiagnostic] {
        &self.diagnostics
    }
    pub fn queries(&self) -> KerMlQueries<'_> {
        queries(&self.snapshot)
    }
    /// Validation of this declared, public, long-name slice only. This does not
    /// certify full KerML constraints, implied library semantics or execution.
    pub fn validate_slice(&self) -> Result<ValidatedModel<'_>, ValidationFailure> {
        if self.syntax.status() == SyntaxStatus::Success && self.diagnostics.is_empty() {
            Ok(ValidatedModel(self))
        } else {
            Err(ValidationFailure {
                syntax_diagnostics: self.syntax.diagnostics().len(),
                semantic_diagnostics: self.diagnostics.len(),
            })
        }
    }
}
pub struct ValidatedModel<'a>(&'a WorkingModel);
impl ValidatedModel<'_> {
    pub fn model(&self) -> &WorkingModel {
        self.0
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct ValidationFailure {
    pub syntax_diagnostics: usize,
    pub semantic_diagnostics: usize,
}
fn queries(snapshot: &Snapshot) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(snapshot, SemanticOptions::default(), BTreeSet::new())
            .expect("pinned KerML descriptors"),
    )
}
fn authored(source: SourceOrigin) -> DeclaredOrigin {
    DeclaredOrigin::Authored {
        source: Some(source),
    }
}
fn reference(id: ElementId) -> SlotValue {
    SlotValue::Scalar(Value::Reference(id))
}
fn text(value: impl Into<String>) -> SlotValue {
    SlotValue::Scalar(Value::String(value.into()))
}

struct Builder {
    base: Snapshot,
    changes: ChangeSet,
    links: BTreeMap<(ElementId, PropertyId), (Vec<Value>, SourceOrigin)>,
}
impl Builder {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry().expect("generated descriptors"),
        ));
        let changes = base.change_set();
        Self {
            base,
            changes,
            links: BTreeMap::new(),
        }
    }
    fn create(&mut self, id: ElementId, class: MetaclassId, origin: &SourceOrigin) {
        self.changes.create(id, class, authored(origin.clone()));
        self.set(id, p::ELEMENT_ELEMENT_ID, text(id.to_string()), origin);
        // Explicit grammar/default facts for this exact slice. No generic
        // filling of arbitrary required properties with guessed values.
        let false_properties = [
            p::ELEMENT_IS_IMPLIED_INCLUDED,
            p::TYPE_IS_ABSTRACT,
            p::TYPE_IS_SUFFICIENT,
            p::FEATURE_IS_COMPOSITE,
            p::FEATURE_IS_CONSTANT,
            p::FEATURE_IS_DERIVED,
            p::FEATURE_IS_END,
            p::FEATURE_IS_ORDERED,
            p::FEATURE_IS_PORTION,
            p::FEATURE_IS_VARIABLE,
            p::RELATIONSHIP_IS_IMPLIED,
        ];
        for property in false_properties {
            if self
                .base
                .model()
                .registry()
                .is_legal(class, property)
                .expect("descriptor")
            {
                self.set(
                    id,
                    property,
                    SlotValue::Scalar(Value::Boolean(false)),
                    origin,
                );
            }
        }
        if class == c::FEATURE {
            self.set(
                id,
                p::FEATURE_IS_UNIQUE,
                SlotValue::Scalar(Value::Boolean(true)),
                origin,
            );
        }
        if self
            .base
            .model()
            .registry()
            .is_subtype(class, c::MEMBERSHIP)
            .expect("descriptor")
        {
            let registry = self.base.model().registry();
            let metamodel::ValueKind::Enumeration(domain) = registry
                .property(p::MEMBERSHIP_VISIBILITY)
                .expect("descriptor")
                .value_kind
            else {
                unreachable!()
            };
            let public = *registry
                .enumeration(domain)
                .expect("domain")
                .literals
                .iter()
                .find(|(_, name)| name.as_str() == "public")
                .expect("public visibility")
                .0;
            self.set(
                id,
                p::MEMBERSHIP_VISIBILITY,
                SlotValue::Scalar(Value::Enumeration(public)),
                origin,
            );
        }
    }
    fn set(
        &mut self,
        id: ElementId,
        property: PropertyId,
        value: SlotValue,
        origin: &SourceOrigin,
    ) {
        self.changes
            .set(id, property, value, authored(origin.clone()));
    }
    fn link(
        &mut self,
        source: ElementId,
        property: PropertyId,
        target: ElementId,
        origin: &SourceOrigin,
    ) {
        self.links
            .entry((source, property))
            .or_insert_with(|| (vec![], origin.clone()))
            .0
            .push(Value::Reference(target));
    }
    fn finish(mut self) -> Result<Snapshot, ModelError> {
        for ((source, property), (values, origin)) in self.links {
            self.changes.set(
                source,
                property,
                SlotValue::Ordered(values),
                authored(origin),
            );
        }
        self.base.apply(&self.changes)
    }
}

struct Pending {
    id: ElementId,
    specific: ElementId,
    kind: ReferenceKind,
    name: QualifiedName,
    origin: SourceOrigin,
}
pub(crate) fn lower(
    syntax: SyntaxDocument,
    previous: Option<&WorkingModel>,
) -> Result<WorkingModel, ModelError> {
    let mut builder = Builder::new();
    let mut identities = BTreeMap::new();
    let mut pending = vec![];
    let root = previous
        .filter(|m| m.syntax.root_id() == syntax.root_id())
        .map(|m| m.root)
        .unwrap_or_default();
    let root_origin = syntax.origin(
        syntax.root_id(),
        ByteRange::new(0, syntax.source().len() as u64).unwrap(),
    );
    builder.create(root, c::NAMESPACE, &root_origin);
    lower_nodes(
        &syntax,
        syntax.nodes(),
        root,
        c::NAMESPACE,
        previous,
        &mut builder,
        &mut identities,
        &mut pending,
    );
    let declarations = builder.finish()?;
    let scopes = pending.iter().map(|r| r.specific).collect();
    let initial = working_queries(&declarations, scopes);
    let mut references: Vec<_> = pending
        .into_iter()
        .map(|r| {
            let resolution = initial.resolve_reference(r.specific, &r.name, expected(r.kind));
            ReferenceAssertion {
                relationship: r.id,
                specific: r.specific,
                kind: r.kind,
                name: r.name,
                origin: r.origin,
                resolution,
            }
        })
        .collect();
    let desired = add_relationships(&declarations, &references)?;
    let snapshot = if let Some(previous) = previous {
        publish(&previous.snapshot, &desired)?
    } else {
        desired
    };
    // Rebind evidence to the actual published snapshot, with unresolved scope
    // obligations in its semantic context. No speculative edges are constructed.
    let scopes = references
        .iter()
        .filter(|r| !matches!(r.resolution.value, Resolution::Resolved(_)))
        .map(|r| r.specific)
        .collect();
    let resolver = working_queries(&snapshot, scopes);
    for r in &mut references {
        r.resolution = resolver.resolve_reference(r.specific, &r.name, expected(r.kind));
    }
    let mut diagnostics = vec![];
    for r in &references {
        for d in &r.resolution.diagnostics {
            diagnostics.push(FrontendDiagnostic {
                code: d.code,
                origin: r.origin.clone(),
                message: d.message.clone(),
            });
        }
    }
    let q = queries(&snapshot);
    for element in snapshot.model().elements() {
        if snapshot
            .model()
            .registry()
            .is_subtype(element.metaclass(), c::TYPE)?
        {
            let result = q.effective_features(element.id());
            for d in result.diagnostics {
                diagnostics.push(FrontendDiagnostic {
                    code: d.code,
                    origin: source_origin(element.origin()),
                    message: d.message,
                });
            }
            if result.completeness != Completeness::Complete && diagnostics.is_empty() {
                diagnostics.push(FrontendDiagnostic {
                    code: "KT_INCOMPLETE",
                    origin: source_origin(element.origin()),
                    message: "Semantic queries are incomplete".into(),
                });
            }
        }
        if snapshot
            .model()
            .registry()
            .is_subtype(element.metaclass(), c::NAMESPACE)?
        {
            let memberships = q.memberships(element.id());
            let mut names = BTreeSet::new();
            for membership in memberships.value {
                if let Some(member) = q.member(membership).value {
                    let record = snapshot.model().element(member).unwrap();
                    if let Some(slot) = record.slot(p::ELEMENT_DECLARED_NAME)
                        && let SlotValue::Scalar(Value::String(name)) = slot.value()
                        && !names.insert(name.clone())
                    {
                        diagnostics.push(FrontendDiagnostic {
                            code: "KT_DUPLICATE_NAME",
                            origin: source_origin(record.origin()),
                            message: format!("Duplicate declared member name {name}"),
                        });
                    }
                }
            }
        }
    }
    Ok(WorkingModel {
        syntax,
        snapshot,
        root,
        identities,
        references,
        diagnostics,
    })
}
fn source_origin(origin: &Origin) -> SourceOrigin {
    let Origin::Declared(DeclaredOrigin::Authored {
        source: Some(source),
    }) = origin
    else {
        unreachable!("frontend authors every record")
    };
    source.clone()
}
fn working_queries(snapshot: &Snapshot, scopes: BTreeSet<ElementId>) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_working_snapshot(
            snapshot,
            SemanticOptions::default(),
            BTreeSet::new(),
            scopes,
        )
        .expect("lowered Types"),
    )
}
fn expected(kind: ReferenceKind) -> MetaclassId {
    match kind {
        ReferenceKind::Specialization | ReferenceKind::Typing => c::TYPE,
        _ => c::FEATURE,
    }
}
fn relation_class(kind: ReferenceKind) -> MetaclassId {
    match kind {
        ReferenceKind::Specialization => c::SPECIALIZATION,
        ReferenceKind::Typing => c::FEATURE_TYPING,
        ReferenceKind::Subsetting => c::SUBSETTING,
        ReferenceKind::Redefinition => c::REDEFINITION,
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_nodes(
    syntax: &SyntaxDocument,
    nodes: &[SyntaxNode],
    owner: ElementId,
    owner_class: MetaclassId,
    previous: Option<&WorkingModel>,
    builder: &mut Builder,
    identities: &mut BTreeMap<SyntaxNodeId, Ids>,
    pending: &mut Vec<Pending>,
) {
    for d in nodes
        .iter()
        .filter_map(Declaration::cast)
        .map(Declaration::syntax)
    {
        if !d.header_valid {
            continue;
        }
        let ids = previous
            .and_then(|m| m.identities.get(&d.id))
            .cloned()
            .unwrap_or_else(|| Ids {
                element: ElementId::new(),
                membership: ElementId::new(),
            });
        let class = match d.kind {
            DeclarationKind::Namespace => c::NAMESPACE,
            DeclarationKind::Type => c::TYPE,
            DeclarationKind::Feature => c::FEATURE,
        };
        let origin = syntax.origin(d.id, d.range);
        builder.create(ids.element, class, &origin);
        if let Some(name) = &d.name {
            builder.set(
                ids.element,
                p::ELEMENT_DECLARED_NAME,
                text(&name.value),
                &syntax.origin(d.id, name.range),
            );
        }
        if d.is_abstract {
            builder.set(
                ids.element,
                p::TYPE_IS_ABSTRACT,
                SlotValue::Scalar(Value::Boolean(true)),
                &syntax.origin(d.id, d.header),
            );
        }
        let membership_class = if class == c::FEATURE && owner_class != c::NAMESPACE {
            c::FEATURE_MEMBERSHIP
        } else {
            c::OWNING_MEMBERSHIP
        };
        builder.create(ids.membership, membership_class, &origin);
        builder.link(
            owner,
            p::ELEMENT_OWNED_RELATIONSHIP,
            ids.membership,
            &origin,
        );
        builder.link(
            ids.membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ids.element,
            &origin,
        );
        lower_references(syntax, d, ids.element, previous, pending);
        lower_nodes(
            syntax,
            &d.children,
            ids.element,
            class,
            previous,
            builder,
            identities,
            pending,
        );
        identities.insert(d.id, ids);
    }
}
fn lower_references(
    syntax: &SyntaxDocument,
    d: &DeclarationSyntax,
    specific: ElementId,
    previous: Option<&WorkingModel>,
    pending: &mut Vec<Pending>,
) {
    for r in &d.references {
        let id = previous
            .and_then(|m| {
                m.references.iter().find(|a| {
                    a.origin.syntax_node == Some(r.id)
                        && m.snapshot.model().element(a.relationship).is_some()
                })
            })
            .map(|a| a.relationship)
            .unwrap_or_default();
        pending.push(Pending {
            id,
            specific,
            kind: r.kind,
            name: QualifiedName {
                absolute: r.absolute,
                segments: r.segments.iter().map(|n| n.value.clone()).collect(),
            },
            origin: syntax.origin(r.id, r.range),
        });
    }
}
fn add_relationships(
    base: &Snapshot,
    references: &[ReferenceAssertion],
) -> Result<Snapshot, ModelError> {
    let mut builder = Builder {
        base: base.clone(),
        changes: base.change_set(),
        links: BTreeMap::new(),
    };
    for r in references {
        let Resolution::Resolved(target) = r.resolution.value else {
            continue;
        };
        let class = relation_class(r.kind);
        builder.create(r.relationship, class, &r.origin);
        let registry = base.model().registry();
        let specific = registry
            .resolve_property(class, p::SPECIALIZATION_SPECIFIC)?
            .expect("specific")
            .id;
        let general = registry
            .resolve_property(class, p::SPECIALIZATION_GENERAL)?
            .expect("general")
            .id;
        builder.set(r.relationship, specific, reference(r.specific), &r.origin);
        builder.set(r.relationship, general, reference(target), &r.origin);
        builder
            .links
            .entry((r.specific, p::ELEMENT_OWNED_RELATIONSHIP))
            .or_insert_with(|| {
                (
                    vec![],
                    source_origin(base.model().element(r.specific).unwrap().origin()),
                )
            });
        builder.link(
            r.specific,
            p::ELEMENT_OWNED_RELATIONSHIP,
            r.relationship,
            &r.origin,
        );
    }
    // Header relationship assertions precede body memberships in the normative
    // ownedRelationship order produced by the concrete grammar.
    for ((owner, property), (values, _)) in &mut builder.links {
        if let Some(slot) = base.model().element(*owner).unwrap().slot(*property) {
            values.extend(slot.value().values().cloned());
        }
    }
    builder.finish()
}
fn publish(previous: &Snapshot, desired: &Snapshot) -> Result<Snapshot, ModelError> {
    let mut changes = previous.change_set();
    for old in previous.model().elements() {
        if desired.model().element(old.id()).is_none() {
            changes.remove(old.id());
        }
    }
    for record in desired.model().elements() {
        let origin = authored(source_origin(record.origin()));
        if let Some(old) = previous.model().element(record.id()) {
            changes.set_origin(record.id(), origin);
            for (property, _) in old.slots() {
                changes.clear(record.id(), property);
            }
        } else {
            changes.create(record.id(), record.metaclass(), origin);
        }
        for (property, slot) in record.slots() {
            changes.set(
                record.id(),
                property,
                slot.value().clone(),
                authored(source_origin(slot.origin())),
            );
        }
    }
    previous.apply(&changes)
}
