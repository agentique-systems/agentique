//! A transient transaction builder. Only its ordinary kernel Snapshot is published.
use super::*;
use agq_kerml::{classes as c, properties as p};
use agq_kerml_syntax::{
    TokenKind,
    production::{Node, Production as P},
};
use agq_kernel::{
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use agq_standard_libraries::LibraryElementRole;
use std::sync::Arc;
#[path = "sysml_construction.rs"]
mod sysml_construction;
use sysml_construction::sysml_class;

pub(crate) fn check_supported_sysml(syntax: &production::Document) -> Result<(), LibraryLoadError> {
    for node in syntax.nodes() {
        sysml_class(node)?;
    }
    Ok(())
}

struct Record {
    class: MetaclassId,
    origin: DeclaredOrigin,
    source: Option<SourceOrigin>,
    slots: BTreeMap<PropertyId, SlotValue>,
}
/// Shared canonical construction input. Authored identities follow reconciled
/// syntax nodes; pinned library identities retain their content/role locators.
pub(crate) struct SourceInput<'a> {
    pub syntax: &'a production::Document,
    pub library: Option<&'a LibraryDocument>,
    pub sysml: bool,
}
impl SourceInput<'_> {
    fn origin(&self, source: SourceOrigin) -> DeclaredOrigin {
        self.library.map_or_else(
            || DeclaredOrigin::Authored {
                source: Some(source),
            },
            LibraryDocument::origin,
        )
    }
    fn element_id(&self, node: Node<'_>, ordinal: u32) -> Result<ElementId, LibraryLoadError> {
        if let Some(library) = self.library {
            Ok(library.element_id(
                node.range(),
                LibraryElementRole::Canonical {
                    role: node.kind().name(),
                    ordinal,
                },
            )?)
        } else {
            Ok(authored_id(node.id(), node.kind().name()))
        }
    }
    fn owned_id(
        &self,
        source: &SourceOrigin,
        owner: ElementId,
        role: &str,
    ) -> Result<ElementId, LibraryLoadError> {
        if let Some(library) = self.library {
            Ok(library.owned_element_id(source.range, owner, role)?)
        } else {
            Ok(ElementId::from_u128(
                uuid::Uuid::new_v5(&uuid::Uuid::from_u128(owner.as_u128()), role.as_bytes())
                    .as_u128(),
            ))
        }
    }
}
fn authored_id(syntax: agq_kernel::SyntaxNodeId, role: &str) -> ElementId {
    ElementId::from_u128(
        uuid::Uuid::new_v5(&uuid::Uuid::from_u128(syntax.as_u128()), role.as_bytes()).as_u128(),
    )
}
struct Builder {
    profile: agq_kerml::BaselineProfile,
    base: Snapshot,
    records: BTreeMap<ElementId, Record>,
    order: Vec<ElementId>,
    references: Vec<PendingLibraryReference>,
    roots: Vec<ElementId>,
    parents: BTreeMap<ElementId, ElementId>,
}
#[derive(Clone, Copy)]
struct Job<'a> {
    node: Node<'a>,
    owner: Option<ElementId>,
    target: Option<PropertyId>,
    expression: bool,
    inline_chain: bool,
}

pub(super) fn construct(
    inputs: &[Input],
    resolved: &BTreeMap<(ElementId, PropertyId), ElementId>,
    profile: agq_kerml::BaselineProfile,
) -> Result<LibraryDraft, LibraryLoadError> {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(profile)
            .map_err(|e| LibraryLoadError::Interpretation(e.to_string()))?,
    ));
    let inputs: Vec<_> = inputs
        .iter()
        .map(|input| SourceInput {
            syntax: &input.syntax,
            library: Some(&input.source),
            sysml: false,
        })
        .collect();
    construct_on(&inputs, resolved, profile, base, None)
}

pub(crate) fn construct_on(
    inputs: &[SourceInput<'_>],
    resolved: &BTreeMap<(ElementId, PropertyId), ElementId>,
    profile: agq_kerml::BaselineProfile,
    base: Snapshot,
    project_root: Option<(ElementId, DeclaredOrigin)>,
) -> Result<LibraryDraft, LibraryLoadError> {
    let mut builder = Builder {
        profile,
        base,
        records: BTreeMap::new(),
        order: vec![],
        references: vec![],
        roots: vec![],
        parents: BTreeMap::new(),
    };
    if let Some((root, origin)) = &project_root {
        builder.create(*root, c::NAMESPACE, origin.clone(), None)?;
        builder.roots.push(*root);
    }
    for input in inputs {
        let mut ordinals = BTreeMap::<(u64, u64, &'static str), u32>::new();
        let mut jobs: Vec<_> = input
            .syntax
            .roots()
            .map(|node| Job {
                node,
                owner: project_root.as_ref().map(|(root, _)| *root),
                target: None,
                expression: false,
                inline_chain: false,
            })
            .collect();
        while let Some(mut job) = jobs.pop() {
            let node = job.node;
            // The printed cast productions append EmptyResultMember even after
            // TypeResultMember. The latter already constructs the expression's
            // typed result. EmptyResultMember is a zero-width default, not a
            // second result (validateExpressionResultParameterMembership).
            // Keep both syntax nodes; see ADR 0013's explicit disposition.
            if node.kind() == P::EmptyResultMember
                && job.owner.is_some_and(|owner| builder.has_result(owner))
            {
                continue;
            }
            if node.kind() == P::OwnedExpression {
                job.expression = true;
            }
            let class = if node.kind() == P::RootNamespace && project_root.is_some() {
                None
            } else if node.kind() == P::Import {
                let declaration = node
                    .child(P::ImportDeclaration)
                    .and_then(|n| n.children().next())
                    .ok_or_else(|| LibraryLoadError::Interpretation("import declaration".into()))?;
                Some(match declaration.kind() {
                    P::MembershipImport => c::MEMBERSHIP_IMPORT,
                    P::NamespaceImport => c::NAMESPACE_IMPORT,
                    _ => return Err(LibraryLoadError::Interpretation("filtered import".into())),
                })
            } else if matches!(node.kind(), P::MembershipImport | P::NamespaceImport)
                || (node.kind() == P::FeatureChain && job.inline_chain)
            {
                None
            } else if input.sysml {
                sysml_class(node)?
            } else {
                vocabulary::class(node)
            };
            if let Some(class) = class {
                let ordinal = ordinals
                    .entry((node.range().start(), node.range().end(), node.kind().name()))
                    .or_default();
                let id = input.element_id(node, *ordinal)?;
                *ordinal += 1;
                builder.create(id, class, input.origin(node.origin()), Some(node.origin()))?;
                if let Some(owner) = job.owner {
                    let owner_class = builder.records[&owner].class;
                    let property = if builder.is_class(owner_class, c::RELATIONSHIP)
                        && (builder.is_class(owner_class, c::OWNING_MEMBERSHIP)
                            || !builder.is_class(class, c::RELATIONSHIP))
                    {
                        p::RELATIONSHIP_OWNED_RELATED_ELEMENT
                    } else {
                        p::ELEMENT_OWNED_RELATIONSHIP
                    };
                    builder.link(owner, property, id);
                    if input.sysml && builder.is_class(class, agq_sysml::classes::USAGE) {
                        builder.usage_composite_default(id, owner)?;
                    }
                    // 8.2 ConnectorEndMember/FlowEndMember establish an end
                    // even when no explicit "end" token occurs. This applies
                    // only to the newly constructed owned Feature. Reference
                    // memberships and inherited projections never mutate their
                    // existing member's end state.
                    if property == p::RELATIONSHIP_OWNED_RELATED_ELEMENT
                        && builder.is_class(owner_class, c::END_FEATURE_MEMBERSHIP)
                        && builder.is_class(class, c::FEATURE)
                    {
                        builder.set(id, p::FEATURE_IS_END, Value::Boolean(true))?;
                    }
                    if builder.is_class(owner_class, c::PARAMETER_MEMBERSHIP)
                        && builder.is_class(class, c::FEATURE)
                    {
                        builder.enumeration(
                            id,
                            p::FEATURE_DIRECTION,
                            if builder.is_class(owner_class, c::RETURN_PARAMETER_MEMBERSHIP) {
                                "out"
                            } else {
                                "in"
                            },
                        )?;
                    }
                    if !builder.is_class(class, c::RELATIONSHIP)
                        && let Some(target) = job.target
                    {
                        builder.set(owner, target, Value::Reference(id))?;
                    }
                    if builder.is_class(class, c::SPECIALIZATION)
                        && builder.is_class(owner_class, c::TYPE)
                        && !matches!(
                            node.kind(),
                            P::Specialization
                                | P::Subclassification
                                | P::FeatureTyping
                                | P::Subsetting
                                | P::Redefinition
                        )
                    {
                        let specific = builder
                            .base
                            .model()
                            .registry()
                            .resolve_property(class, p::SPECIALIZATION_SPECIFIC)
                            .expect("specific property")
                            .expect("specialization");
                        if !specific.derived {
                            builder.set(id, p::SPECIALIZATION_SPECIFIC, Value::Reference(owner))?;
                        }
                    }
                    for (relation, source_property, source_class) in [
                        (c::CONJUGATION, p::CONJUGATION_CONJUGATED_TYPE, c::TYPE),
                        (
                            c::FEATURE_INVERTING,
                            p::FEATURE_INVERTING_FEATURE_INVERTED,
                            c::FEATURE,
                        ),
                        (c::DISJOINING, p::DISJOINING_TYPE_DISJOINED, c::TYPE),
                        (
                            c::TYPE_FEATURING,
                            p::TYPE_FEATURING_FEATURE_OF_TYPE,
                            c::FEATURE,
                        ),
                    ] {
                        if builder.is_class(class, relation)
                            && builder.is_class(owner_class, source_class)
                            && builder
                                .base
                                .model()
                                .registry()
                                .resolve_property(class, source_property)
                                .map_err(agq_kernel::ModelError::from)?
                                .is_some_and(|property| !property.derived)
                            && !matches!(
                                node.kind(),
                                P::Conjugation
                                    | P::FeatureInverting
                                    | P::Disjoining
                                    | P::TypeFeaturing
                            )
                        {
                            builder.set(id, source_property, Value::Reference(owner))?;
                        }
                    }
                } else {
                    builder.roots.push(id);
                }
                job.owner = Some(id);
                job.target = target_property(class);
                job.inline_chain = false;
            }
            if let Some(owner) = job.owner {
                if input.sysml {
                    builder.interpret_sysml(node, owner, &mut job)?;
                }
                builder.interpret(node, owner, &mut job)?;
            }
            let children: Vec<_> = node.children().collect();
            let mut endpoint = 0;
            let mut next = vec![];
            for child in children {
                let mut child_job = Job { node: child, ..job };
                if matches!(
                    child.kind(),
                    P::QualifiedName | P::FeatureChain | P::OwnedFeatureChain
                ) {
                    if let Some(property) = explicit_endpoint_property(node.kind(), endpoint) {
                        child_job.target = Some(property);
                    }
                    endpoint += 1;
                }
                next.push(child_job);
            }
            jobs.extend(next.into_iter().rev());
        }
    }
    builder.sysml_structural_completion(inputs)?;
    for ((id, property), target) in resolved {
        builder.set(*id, *property, Value::Reference(*target))?;
    }
    builder.expression_results(inputs)?;
    builder.publish()
}

fn explicit_endpoint_property(production: P, ordinal: usize) -> Option<PropertyId> {
    let endpoints = match production {
        P::Subclassification | P::FeatureTyping => {
            [p::SPECIALIZATION_SPECIFIC, p::SPECIALIZATION_GENERAL]
        }
        P::Conjugation => [p::CONJUGATION_CONJUGATED_TYPE, p::CONJUGATION_ORIGINAL_TYPE],
        P::Disjoining => [p::DISJOINING_TYPE_DISJOINED, p::DISJOINING_DISJOINING_TYPE],
        P::FeatureInverting => [
            p::FEATURE_INVERTING_FEATURE_INVERTED,
            p::FEATURE_INVERTING_INVERTING_FEATURE,
        ],
        P::TypeFeaturing => [
            p::TYPE_FEATURING_FEATURE_OF_TYPE,
            p::TYPE_FEATURING_FEATURING_TYPE,
        ],
        _ => return None,
    };
    endpoints.get(ordinal).copied()
}

impl Builder {
    fn has_result(&self, expression: ElementId) -> bool {
        self.records[&expression].slots.get(&p::ELEMENT_OWNED_RELATIONSHIP)
            .is_some_and(|slot| slot.values().any(|v| matches!(v, Value::Reference(id) if self.is_class(self.records[id].class,c::RETURN_PARAMETER_MEMBERSHIP))))
    }
    fn expression_results(&mut self, inputs: &[SourceInput<'_>]) -> Result<(), LibraryLoadError> {
        let productions: BTreeMap<_, _> = inputs
            .iter()
            .flat_map(|input| input.syntax.nodes().map(|node| (node.id(), node.kind())))
            .collect();
        let expressions: Vec<_> = self
            .order
            .iter()
            .copied()
            .filter(|id| self.is_class(self.records[id].class, c::EXPRESSION))
            .collect();
        for expression in expressions {
            if self.has_result(expression) {
                continue;
            }
            if self.records[&expression]
                .source
                .as_ref()
                .and_then(|source| source.syntax_node)
                .and_then(|id| productions.get(&id))
                .is_some_and(|kind| {
                    matches!(kind, P::Expression | P::BooleanExpression | P::Invariant)
                })
            {
                // A declared Expression may inherit its ReturnParameterMembership.
                // Creating an unnamed local result would suppress that inherited
                // identity and its name (e.g. Performances::trueEvaluations).
                continue;
            }
            // validateExpressionResultParameterMembership and the result-owning
            // constraints require a result even for abbreviated literal/primary
            // notation. These are structural children, not executable evaluation.
            let source = self.records[&expression]
                .source
                .clone()
                .expect("expression source");
            let document = &inputs
                .iter()
                .find(|i| i.syntax.document() == source.document)
                .expect("source document");
            let membership = document.owned_id(&source, expression, "result-membership")?;
            let feature = document.owned_id(&source, expression, "result-feature")?;
            self.create(
                membership,
                c::RETURN_PARAMETER_MEMBERSHIP,
                document.origin(source.clone()),
                Some(source.clone()),
            )?;
            self.create(
                feature,
                c::FEATURE,
                document.origin(source.clone()),
                Some(source),
            )?;
            self.set(membership, p::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true))?;
            self.enumeration(feature, p::FEATURE_DIRECTION, "out")?;
            self.link(expression, p::ELEMENT_OWNED_RELATIONSHIP, membership);
            self.link(membership, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, feature);
            self.set(
                expression,
                p::ELEMENT_IS_IMPLIED_INCLUDED,
                Value::Boolean(true),
            )?;
        }
        Ok(())
    }
    fn is_class(&self, actual: MetaclassId, expected: MetaclassId) -> bool {
        self.base
            .model()
            .registry()
            .is_subtype(actual, expected)
            .expect("class")
    }
    fn create(
        &mut self,
        id: ElementId,
        class: MetaclassId,
        origin: DeclaredOrigin,
        source: Option<SourceOrigin>,
    ) -> Result<(), LibraryLoadError> {
        if self
            .records
            .insert(
                id,
                Record {
                    class,
                    origin,
                    source,
                    slots: BTreeMap::new(),
                },
            )
            .is_some()
        {
            return Err(LibraryLoadError::Interpretation(
                "duplicate canonical output locator".into(),
            ));
        }
        self.order.push(id);
        self.set(id, p::ELEMENT_ELEMENT_ID, Value::String(id.to_string()))?;
        // Exact default literals in pinned KerML 1.0 XMI, not guesses for arbitrary properties.
        for property in [
            p::ELEMENT_IS_IMPLIED_INCLUDED,
            p::RELATIONSHIP_IS_IMPLIED,
            p::TYPE_IS_ABSTRACT,
            p::TYPE_IS_SUFFICIENT,
            p::FEATURE_IS_COMPOSITE,
            p::FEATURE_IS_CONSTANT,
            p::FEATURE_IS_DERIVED,
            p::FEATURE_IS_END,
            p::FEATURE_IS_ORDERED,
            p::FEATURE_IS_PORTION,
            p::FEATURE_IS_VARIABLE,
            p::FEATURE_VALUE_IS_DEFAULT,
            p::FEATURE_VALUE_IS_INITIAL,
            p::INVARIANT_IS_NEGATED,
            p::LIBRARY_PACKAGE_IS_STANDARD,
            p::IMPORT_IS_IMPORT_ALL,
            p::IMPORT_IS_RECURSIVE,
        ] {
            if self
                .base
                .model()
                .registry()
                .resolve_property(class, property)
                .expect("property")
                .is_some_and(|property| !property.derived)
            {
                self.set(id, property, Value::Boolean(false))?;
            }
        }
        if self.is_class(class, c::FEATURE) {
            self.set(id, p::FEATURE_IS_UNIQUE, Value::Boolean(true))?;
        }
        if self.is_class(class, c::MEMBERSHIP) {
            self.enumeration(id, p::MEMBERSHIP_VISIBILITY, "public")?;
        }
        if self.is_class(class, c::IMPORT) {
            self.enumeration(id, p::IMPORT_VISIBILITY, "private")?;
        }
        Ok(())
    }
    fn set(
        &mut self,
        id: ElementId,
        property: PropertyId,
        value: Value,
    ) -> Result<(), LibraryLoadError> {
        let class = self.records[&id].class;
        let descriptor = self
            .base
            .model()
            .registry()
            .resolve_property(class, property)
            .map_err(agq_kernel::ModelError::from)?
            .ok_or_else(|| {
                LibraryLoadError::Interpretation(format!("illegal property {property} on {class}"))
            })?;
        if descriptor.derived {
            return Err(LibraryLoadError::Interpretation(format!(
                "grammar attempted derived property {}",
                descriptor.name
            )));
        }
        self.records
            .get_mut(&id)
            .unwrap()
            .slots
            .insert(descriptor.id, SlotValue::Scalar(value));
        Ok(())
    }
    fn enumeration(
        &mut self,
        id: ElementId,
        property: PropertyId,
        name: &str,
    ) -> Result<(), LibraryLoadError> {
        let registry = self.base.model().registry();
        let ValueKind::Enumeration(domain) =
            registry.property(property).expect("property").value_kind
        else {
            unreachable!()
        };
        let literal = *registry
            .enumeration(domain)
            .expect("domain")
            .literals
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .ok_or_else(|| LibraryLoadError::Interpretation(format!("enumeration {name}")))?
            .0;
        self.set(id, property, Value::Enumeration(literal))
    }
    fn link(&mut self, owner: ElementId, property: PropertyId, target: ElementId) {
        if matches!(
            property,
            p::ELEMENT_OWNED_RELATIONSHIP | p::RELATIONSHIP_OWNED_RELATED_ELEMENT
        ) {
            self.parents.insert(target, owner);
        }
        let slot = self
            .records
            .get_mut(&owner)
            .unwrap()
            .slots
            .entry(property)
            .or_insert_with(|| SlotValue::Ordered(vec![]));
        let SlotValue::Ordered(values) = slot else {
            unreachable!()
        };
        values.push(Value::Reference(target));
    }
    fn interpret(
        &mut self,
        node: Node<'_>,
        id: ElementId,
        job: &mut Job<'_>,
    ) -> Result<(), LibraryLoadError> {
        let class = self.records[&id].class;
        let words: Vec<_> = node
            .tokens()
            .filter(|t| !t.kind.is_trivia())
            .map(|t| node_text(node, t.range))
            .collect();
        match node.kind() {
            P::Identification | P::FeatureIdentification => {
                let names: Vec<_> = node.names().collect();
                let short = words.first() == Some(&"<");
                if short && let Some(name) = names.first() {
                    self.set(
                        id,
                        p::ELEMENT_DECLARED_SHORT_NAME,
                        Value::String(name.value.clone()),
                    )?;
                }
                if let Some(name) = names.get(usize::from(short)) {
                    self.set(
                        id,
                        p::ELEMENT_DECLARED_NAME,
                        Value::String(name.value.clone()),
                    )?;
                }
            }
            P::AliasMember => {
                // Names are obtained only from the already recognized AliasMember
                // header before its QualifiedName child, including unrestricted names.
                let target_start = node
                    .child(P::QualifiedName)
                    .map(|n| n.range().start())
                    .unwrap_or(u64::MAX);
                let names: Vec<_> = node
                    .names()
                    .filter(|n| n.range.end() < target_start)
                    .collect();
                let short = words.iter().take_while(|&&w| w != "for").any(|&w| w == "<");
                if short && let Some(name) = names.first() {
                    self.set(
                        id,
                        p::MEMBERSHIP_MEMBER_SHORT_NAME,
                        Value::String(name.value.clone()),
                    )?;
                }
                if let Some(name) = names.get(usize::from(short)) {
                    self.set(
                        id,
                        p::MEMBERSHIP_MEMBER_NAME,
                        Value::String(name.value.clone()),
                    )?;
                }
            }
            P::VisibilityIndicator => self.enumeration(
                id,
                if self.is_class(class, c::IMPORT) {
                    p::IMPORT_VISIBILITY
                } else {
                    p::MEMBERSHIP_VISIBILITY
                },
                node.text(),
            )?,
            P::FeatureDirection => self.enumeration(id, p::FEATURE_DIRECTION, node.text())?,
            P::TypePrefix | P::BasicFeaturePrefix | P::EndFeaturePrefix | P::MultiplicityPart => {
                for (word, property) in [
                    ("abstract", p::TYPE_IS_ABSTRACT),
                    ("derived", p::FEATURE_IS_DERIVED),
                    ("composite", p::FEATURE_IS_COMPOSITE),
                    ("portion", p::FEATURE_IS_PORTION),
                    ("var", p::FEATURE_IS_VARIABLE),
                    ("const", p::FEATURE_IS_CONSTANT),
                    ("end", p::FEATURE_IS_END),
                    ("ordered", p::FEATURE_IS_ORDERED),
                ] {
                    if words.contains(&word) {
                        self.set(id, property, Value::Boolean(true))?;
                    }
                }
                if words.contains(&"const") {
                    self.set(id, p::FEATURE_IS_VARIABLE, Value::Boolean(true))?;
                }
                if words.contains(&"nonunique") {
                    self.set(id, p::FEATURE_IS_UNIQUE, Value::Boolean(false))?;
                }
            }
            P::TypeDeclaration | P::ClassifierDeclaration | P::FeatureDeclaration => {
                if words.first() == Some(&"all") {
                    self.set(id, p::TYPE_IS_SUFFICIENT, Value::Boolean(true))?;
                }
            }
            P::LibraryPackage => self.set(
                id,
                p::LIBRARY_PACKAGE_IS_STANDARD,
                Value::Boolean(words.first() == Some(&"standard")),
            )?,
            P::Import => {
                if words
                    .iter()
                    .take_while(|&&w| w != "::")
                    .any(|&w| w == "all")
                {
                    self.set(id, p::IMPORT_IS_IMPORT_ALL, Value::Boolean(true))?;
                }
                if words.contains(&"**") {
                    self.set(id, p::IMPORT_IS_RECURSIVE, Value::Boolean(true))?;
                }
            }
            P::Invariant => {
                if words
                    .iter()
                    .take_while(|&&w| w != "{")
                    .any(|&w| w == "false")
                {
                    self.set(id, p::INVARIANT_IS_NEGATED, Value::Boolean(true))?;
                }
            }
            P::FeatureValue => {
                if words.first() == Some(&"default") {
                    self.set(id, p::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(true))?;
                }
                if words.first() == Some(&":=") || words.get(1) == Some(&":=") {
                    self.set(id, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(true))?;
                }
            }
            P::SpecificType => job.target = Some(p::SPECIALIZATION_SPECIFIC),
            P::GeneralType => job.target = Some(p::SPECIALIZATION_GENERAL),
            P::ChainingPart => job.inline_chain = true,
            P::QualifiedName => {
                let property = job.target.ok_or_else(|| {
                    LibraryLoadError::Interpretation(format!(
                        "unattached qualified name {} in {:?}",
                        node.text(),
                        class
                    ))
                })?;
                let property = self
                    .base
                    .model()
                    .registry()
                    .resolve_property(class, property)
                    .map_err(agq_kernel::ModelError::from)?
                    .ok_or_else(|| LibraryLoadError::Interpretation("reference property".into()))?;
                let ValueKind::Reference(expected) = property.value_kind else {
                    return Err(LibraryLoadError::Interpretation(
                        "nonreference target".into(),
                    ));
                };
                self.references.push(PendingLibraryReference {
                    relationship: id,
                    property: property.id,
                    expected,
                    name: QualifiedName {
                        absolute: words.first() == Some(&"$"),
                        segments: node.names().map(|n| n.value).collect(),
                    },
                    membership_target: class == c::MEMBERSHIP_IMPORT,
                    executable_expression: job.expression,
                    origin: node.origin(),
                });
            }
            P::Comment | P::Documentation => {
                let body = node
                    .tokens()
                    .find(|t| t.kind == TokenKind::Comment)
                    .expect("comment production");
                self.set(
                    id,
                    p::COMMENT_BODY,
                    Value::String(comment_body(node_text(node, body.range))),
                )?;
            }
            P::LiteralBoolean => self.set(
                id,
                p::LITERAL_BOOLEAN_VALUE,
                Value::Boolean(node.text() == "true"),
            )?,
            P::LiteralInteger => self.set(
                id,
                p::LITERAL_INTEGER_VALUE,
                Value::Integer(
                    node.text()
                        .parse()
                        .map_err(|_| LibraryLoadError::Interpretation("integer literal".into()))?,
                ),
            )?,
            P::LiteralReal => self.set(
                id,
                p::LITERAL_RATIONAL_VALUE,
                Value::Real(
                    words
                        .join("")
                        .parse()
                        .map_err(|_| LibraryLoadError::Interpretation("real literal".into()))?,
                ),
            )?,
            P::LiteralString => {
                return Err(LibraryLoadError::Interpretation(
                    "string literal unescaping".into(),
                ));
            }
            P::BinaryOperator
            | P::UnaryOperator
            | P::ConditionalBinaryOperator
            | P::ClassificationTestOperator
            | P::CastOperator
            | P::MetaclassificationTestOperator
            | P::MetaCastOperator => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String(node.text().into()),
            )?,
            P::ConditionalExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("if".into()),
            )?,
            P::ExtentExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("all".into()),
            )?,
            P::SequenceOperatorExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String(",".into()),
            )?,
            P::BracketExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("[".into()),
            )?,
            P::FeatureChainExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String(".".into()),
            )?,
            P::IndexExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("#".into()),
            )?,
            P::CollectExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("collect".into()),
            )?,
            P::SelectExpression => self.set(
                id,
                p::OPERATOR_EXPRESSION_OPERATOR,
                Value::String("select".into()),
            )?,
            _ => {}
        }
        Ok(())
    }
    fn publish(self) -> Result<LibraryDraft, LibraryLoadError> {
        let registry = self.base.model().registry();
        let reference_sources: BTreeMap<_, _> = self
            .references
            .iter()
            .map(|r| ((r.relationship, r.property), &r.origin))
            .collect();
        let mut changes = self.base.change_set();
        let mut source_map = BTreeMap::new();
        for id in &self.order {
            let record = &self.records[id];
            changes.create(*id, record.class, record.origin.clone());
            if let Some(source) = &record.source {
                source_map.insert(FactKey::Element(*id), source.clone());
            }
            for (&property, slot) in &record.slots {
                if !registry
                    .supports_slot_storage(property)
                    .map_err(agq_kernel::ModelError::from)?
                {
                    let descriptor = registry
                        .property(property)
                        .map_err(agq_kernel::ModelError::from)?;
                    let association = descriptor.association.ok_or_else(|| {
                        LibraryLoadError::Interpretation("unsupported nonassociation slot".into())
                    })?;
                    let opposite = *descriptor
                        .opposite_ends
                        .first()
                        .expect("binary association");
                    let SlotValue::Scalar(Value::Reference(target)) = slot else {
                        return Err(LibraryLoadError::Interpretation(
                            "non-scalar occurrence lowering".into(),
                        ));
                    };
                    let link = agq_kernel::AssociationOccurrenceId::from_u128(
                        uuid::Uuid::new_v5(
                            &uuid::Uuid::from_u128(0xa47b82f49e5e5e6ca22f42bac43cc401),
                            format!("library-occurrence/1:{id}:{property}:{target}").as_bytes(),
                        )
                        .as_u128(),
                    );
                    changes.link(
                        link,
                        association,
                        BTreeMap::from([(property, *target), (opposite, *id)]),
                        BTreeMap::new(),
                        record.origin.clone(),
                    );
                    if let Some(source) = &record.source {
                        source_map.insert(FactKey::AssociationOccurrence(link), source.clone());
                    }
                    continue;
                }
                let value = slot.clone();
                changes.set(*id, property, value, record.origin.clone());
                if let Some(source) = reference_sources
                    .get(&(*id, property))
                    .copied()
                    .or(record.source.as_ref())
                {
                    source_map.insert(
                        FactKey::Property {
                            element: *id,
                            property,
                        },
                        source.clone(),
                    );
                }
            }
        }
        let candidate = self.base.preview(&changes)?;
        Ok(LibraryDraft {
            base: self.base,
            profile: self.profile,
            candidate,
            source_map,
            roots: self.roots,
            references: self.references,
            superseded_references: vec![],
        })
    }
}

fn target_property(class: MetaclassId) -> Option<PropertyId> {
    Some(match class {
        c::MEMBERSHIP => p::MEMBERSHIP_MEMBER_ELEMENT,
        c::MEMBERSHIP_IMPORT => p::MEMBERSHIP_IMPORT_IMPORTED_MEMBERSHIP,
        c::NAMESPACE_IMPORT => p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE,
        c::SPECIALIZATION
        | c::SUBCLASSIFICATION
        | c::FEATURE_TYPING
        | c::SUBSETTING
        | c::REDEFINITION
        | c::REFERENCE_SUBSETTING
        | c::CROSS_SUBSETTING => p::SPECIALIZATION_GENERAL,
        c::CONJUGATION => p::CONJUGATION_ORIGINAL_TYPE,
        c::DISJOINING => p::DISJOINING_DISJOINING_TYPE,
        c::UNIONING => p::UNIONING_UNIONING_TYPE,
        c::INTERSECTING => p::INTERSECTING_INTERSECTING_TYPE,
        c::DIFFERENCING => p::DIFFERENCING_DIFFERENCING_TYPE,
        c::FEATURE_CHAINING => p::FEATURE_CHAINING_CHAINING_FEATURE,
        c::FEATURE_INVERTING => p::FEATURE_INVERTING_INVERTING_FEATURE,
        c::TYPE_FEATURING => p::TYPE_FEATURING_FEATURING_TYPE,
        _ => return None,
    })
}
fn node_text(node: Node<'_>, range: agq_kernel::provenance::ByteRange) -> &str {
    &node.text()[(range.start() - node.range().start()) as usize
        ..(range.end() - node.range().start()) as usize]
}
fn comment_body(text: &str) -> String {
    // KerML 1.0 8.2.3.3.2: remove delimiters and normalize line decoration only.
    let body = &text[2..text.len() - 2];
    let initial = body.trim_start_matches([' ', '\t', '\r']);
    let body = initial.strip_prefix('\n').unwrap_or(body);
    body.split_inclusive('\n')
        .enumerate()
        .map(|(i, line)| {
            if i == 0 && !initial.starts_with('\n') {
                line
            } else {
                let line = line.trim_start_matches([' ', '\t']);
                if let Some(line) = line.strip_prefix('*') {
                    line.strip_prefix(' ').unwrap_or(line)
                } else {
                    line
                }
            }
        })
        .collect()
}
