//! Independent command construction compared with textual current-graph answers.
//! No standard publication, semantic producer, or closure certificate is assumed.
use super::*;
use agq_kernel::{AssociationOccurrenceId, ChangeSet, MetaclassId, PropertyId};
use std::collections::BTreeMap;

struct VehicleBuilder {
    base: Snapshot,
    changes: ChangeSet,
    next: u128,
    names: BTreeMap<&'static str, ElementId>,
    owned: BTreeMap<ElementId, Vec<Value>>,
}

impl VehicleBuilder {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        Self::on(base)
    }

    fn on(base: Snapshot) -> Self {
        Self {
            changes: base.change_set(),
            base,
            next: 910_000,
            names: BTreeMap::new(),
            owned: BTreeMap::new(),
        }
    }

    fn scalar(&mut self, element: ElementId, property: PropertyId, value: Value) {
        self.changes.set(
            element,
            property,
            SlotValue::Scalar(value),
            DeclaredOrigin::Authored { source: None },
        );
    }

    fn enumeration(&mut self, element: ElementId, property: PropertyId, name: &str) {
        let registry = self.base.model().registry();
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) =
            registry.property(property).unwrap().value_kind
        else {
            panic!("expected enumeration for {property}");
        };
        let literal = *registry
            .enumeration(domain)
            .unwrap()
            .literals
            .iter()
            .find(|(_, literal)| literal.as_str() == name)
            .unwrap()
            .0;
        self.scalar(element, property, Value::Enumeration(literal));
    }

    fn create(&mut self, class: MetaclassId, name: Option<&'static str>) -> ElementId {
        let id = ElementId::from_u128(self.next);
        self.next += 1;
        self.changes
            .create(id, class, DeclaredOrigin::Authored { source: None });
        self.scalar(id, p::ELEMENT_ELEMENT_ID, Value::String(id.to_string()));
        // Explicit authored fixture defaults; descriptor lookup only determines
        // which stored properties the chosen metaclass supports.
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
            p::INVARIANT_IS_NEGATED,
            p::LIBRARY_PACKAGE_IS_STANDARD,
            sp::DEFINITION_IS_VARIATION,
            sp::USAGE_IS_VARIATION,
            sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL,
            sp::OCCURRENCE_USAGE_IS_INDIVIDUAL,
            sp::STATE_DEFINITION_IS_PARALLEL,
            sp::STATE_USAGE_IS_PARALLEL,
        ] {
            if let Some(descriptor) = self
                .base
                .model()
                .registry()
                .resolve_property(class, property)
                .unwrap()
                .filter(|descriptor| !descriptor.derived)
            {
                self.scalar(id, descriptor.id, Value::Boolean(false));
            }
        }
        if self
            .base
            .model()
            .registry()
            .is_subtype(class, c::FEATURE)
            .unwrap()
        {
            self.scalar(id, p::FEATURE_IS_UNIQUE, Value::Boolean(true));
        }
        if self
            .base
            .model()
            .registry()
            .is_subtype(class, c::MEMBERSHIP)
            .unwrap()
        {
            self.enumeration(id, p::MEMBERSHIP_VISIBILITY, "public");
        }
        if let Some(name) = name {
            assert!(self.names.insert(name, id).is_none());
            self.scalar(id, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        }
        id
    }

    fn member(&mut self, owner: ElementId, member: ElementId, class: MetaclassId) -> ElementId {
        let membership = self.create(class, None);
        self.owned
            .entry(owner)
            .or_default()
            .push(Value::Reference(membership));
        self.changes.set(
            membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(member)]),
            DeclaredOrigin::Authored { source: None },
        );
        membership
    }

    fn reference(&mut self, source: ElementId, property: PropertyId, target: ElementId) {
        let registry = self.base.model().registry();
        if registry.supports_slot_storage(property).unwrap() {
            self.scalar(source, property, Value::Reference(target));
        } else {
            let descriptor = registry.property(property).unwrap();
            self.changes.link(
                AssociationOccurrenceId::from_u128(self.next),
                descriptor.association.unwrap(),
                BTreeMap::from([
                    (property, target),
                    (*descriptor.opposite_ends.first().unwrap(), source),
                ]),
                BTreeMap::new(),
                DeclaredOrigin::Authored { source: None },
            );
            self.next += 1;
        }
    }

    fn relationship(
        &mut self,
        class: MetaclassId,
        source: ElementId,
        target: ElementId,
        source_property: Option<PropertyId>,
        target_property: PropertyId,
    ) {
        let relationship = self.create(class, None);
        self.owned
            .entry(source)
            .or_default()
            .push(Value::Reference(relationship));
        if let Some(property) = source_property {
            self.reference(relationship, property, source);
        }
        self.reference(relationship, target_property, target);
    }

    fn finish(mut self) -> Snapshot {
        for (owner, relationships) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(relationships),
                DeclaredOrigin::Authored { source: None },
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}

fn programmatic_vehicle() -> Snapshot {
    let mut builder = VehicleBuilder::new();
    let mobility = builder.create(c::PACKAGE, Some("Mobility"));
    for (name, class) in [
        ("Quantity", s::ATTRIBUTE_DEFINITION),
        ("Fuel", s::ITEM_DEFINITION),
        ("FuelPort", s::PORT_DEFINITION),
        ("MeteredFuelPort", s::PORT_DEFINITION),
        ("Engine", s::PART_DEFINITION),
        ("TurboEngine", s::PART_DEFINITION),
        ("FuelTank", s::PART_DEFINITION),
        ("FuelLine", s::CONNECTION_DEFINITION),
        ("Drive", s::ACTION_DEFINITION),
        ("Ready", s::STATE_DEFINITION),
        ("Vehicle", s::PART_DEFINITION),
        ("SportsCar", s::PART_DEFINITION),
        ("RoadReady", s::REQUIREMENT_DEFINITION),
    ] {
        let definition = builder.create(class, Some(name));
        builder.member(mobility, definition, c::OWNING_MEMBERSHIP);
    }
    for (owner, name, class, definition, composite) in [
        ("FuelPort", "fuel", s::ITEM_USAGE, "Fuel", false),
        (
            "MeteredFuelPort",
            "flowRate",
            s::ATTRIBUTE_USAGE,
            "Quantity",
            false,
        ),
        ("Engine", "power", s::ATTRIBUTE_USAGE, "Quantity", false),
        ("Engine", "fuelIn", s::PORT_USAGE, "FuelPort", true),
        (
            "TurboEngine",
            "boost",
            s::ATTRIBUTE_USAGE,
            "Quantity",
            false,
        ),
        ("FuelTank", "contents", s::ITEM_USAGE, "Fuel", true),
        ("FuelTank", "fuelOut", s::PORT_USAGE, "FuelPort", true),
        ("FuelLine", "leftEnd", s::PORT_USAGE, "FuelPort", false),
        ("FuelLine", "rightEnd", s::PORT_USAGE, "FuelPort", false),
        ("Drive", "requestedFuel", s::ITEM_USAGE, "Fuel", false),
        ("Drive", "deliveredFuel", s::ITEM_USAGE, "Fuel", false),
        ("Vehicle", "engine", s::PART_USAGE, "Engine", true),
        ("Vehicle", "tank", s::PART_USAGE, "FuelTank", true),
        ("Vehicle", "mass", s::ATTRIBUTE_USAGE, "Quantity", false),
        ("Vehicle", "supply", s::PORT_USAGE, "MeteredFuelPort", true),
        ("Vehicle", "returnFlow", s::PORT_USAGE, "FuelPort", true),
        ("Vehicle", "fuelLine", s::CONNECTION_USAGE, "FuelLine", true),
        ("Vehicle", "driving", s::ACTION_USAGE, "Drive", true),
        ("Vehicle", "operatingState", s::STATE_USAGE, "Ready", true),
        (
            "SportsCar",
            "performanceEngine",
            s::PART_USAGE,
            "TurboEngine",
            true,
        ),
    ] {
        let usage = builder.create(class, Some(name));
        builder.member(builder.names[owner], usage, c::FEATURE_MEMBERSHIP);
        builder.scalar(usage, p::FEATURE_IS_COMPOSITE, Value::Boolean(composite));
        builder.relationship(
            c::FEATURE_TYPING,
            usage,
            builder.names[definition],
            Some(p::FEATURE_TYPING_TYPED_FEATURE),
            p::FEATURE_TYPING_TYPE,
        );
    }
    for (name, direction) in [
        ("fuel", "out"),
        ("requestedFuel", "in"),
        ("deliveredFuel", "out"),
    ] {
        builder.enumeration(builder.names[name], p::FEATURE_DIRECTION, direction);
    }
    for name in ["leftEnd", "rightEnd"] {
        builder.scalar(builder.names[name], p::FEATURE_IS_END, Value::Boolean(true));
    }
    for (specific, general) in [
        ("MeteredFuelPort", "FuelPort"),
        ("TurboEngine", "Engine"),
        ("SportsCar", "Vehicle"),
    ] {
        builder.relationship(
            c::SUBCLASSIFICATION,
            builder.names[specific],
            builder.names[general],
            Some(p::SUBCLASSIFICATION_SUBCLASSIFIER),
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    builder.relationship(
        c::REDEFINITION,
        builder.names["performanceEngine"],
        builder.names["engine"],
        Some(p::REDEFINITION_REDEFINING_FEATURE),
        p::REDEFINITION_REDEFINED_FEATURE,
    );

    // Each connection end owns an actual ReferenceSubsetting to an existing port.
    for endpoint in ["supply", "returnFlow"] {
        let end = builder.create(s::REFERENCE_USAGE, None);
        builder.member(builder.names["fuelLine"], end, c::END_FEATURE_MEMBERSHIP);
        builder.scalar(end, p::FEATURE_IS_END, Value::Boolean(true));
        builder.relationship(
            c::REFERENCE_SUBSETTING,
            end,
            builder.names[endpoint],
            None,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
    for (name, kind) in [
        ("entering", "entry"),
        ("running", "do"),
        ("leaving", "exit"),
    ] {
        let action = builder.create(s::PERFORM_ACTION_USAGE, Some(name));
        builder.scalar(action, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        let membership = builder.member(
            builder.names["Ready"],
            action,
            s::STATE_SUBACTION_MEMBERSHIP,
        );
        builder.enumeration(membership, sp::STATE_SUBACTION_MEMBERSHIP_KIND, kind);
        if name == "running" {
            builder.relationship(
                c::FEATURE_TYPING,
                action,
                builder.names["Drive"],
                Some(p::FEATURE_TYPING_TYPED_FEATURE),
                p::FEATURE_TYPING_TYPE,
            );
        }
    }
    let subject = builder.create(s::REFERENCE_USAGE, Some("checkedVehicle"));
    builder.member(builder.names["RoadReady"], subject, s::SUBJECT_MEMBERSHIP);
    builder.enumeration(subject, p::FEATURE_DIRECTION, "in");
    builder.relationship(
        c::FEATURE_TYPING,
        subject,
        builder.names["Vehicle"],
        Some(p::FEATURE_TYPING_TYPED_FEATURE),
        p::FEATURE_TYPING_TYPE,
    );
    let constraint = builder.create(s::CONSTRAINT_USAGE, Some("inspectionPassed"));
    let membership = builder.member(
        builder.names["RoadReady"],
        constraint,
        s::REQUIREMENT_CONSTRAINT_MEMBERSHIP,
    );
    builder.enumeration(
        membership,
        sp::REQUIREMENT_CONSTRAINT_MEMBERSHIP_KIND,
        "requirement",
    );
    let literal = builder.create(c::LITERAL_BOOLEAN, None);
    builder.scalar(literal, p::LITERAL_BOOLEAN_VALUE, Value::Boolean(true));
    builder.member(constraint, literal, c::RESULT_EXPRESSION_MEMBERSHIP);
    let result = builder.create(c::FEATURE, None);
    builder.member(literal, result, c::RETURN_PARAMETER_MEMBERSHIP);
    builder.enumeration(result, p::FEATURE_DIRECTION, "out");
    builder.finish()
}

/// Independently authored architecture slice: these commands do not read SysML
/// syntax, a textual snapshot, or its IDs. The full model is checked separately.
pub(super) fn programmatic_platform() -> Snapshot {
    programmatic_platform_with_builder(VehicleBuilder::new())
}

pub(super) fn programmatic_platform_on(base: Snapshot) -> Snapshot {
    programmatic_platform_with_builder(VehicleBuilder::on(base))
}

fn programmatic_platform_with_builder(mut builder: VehicleBuilder) -> Snapshot {
    let root = builder.create(c::NAMESPACE, None);
    for package in [
        "ArchitectureContracts",
        "LanguageArchitecture",
        "PlatformArchitecture",
    ] {
        let id = builder.create(c::PACKAGE, Some(package));
        builder.member(root, id, c::OWNING_MEMBERSHIP);
    }
    for (package, name, class) in [
        (
            "ArchitectureContracts",
            "RevisionNumber",
            s::ATTRIBUTE_DEFINITION,
        ),
        ("ArchitectureContracts", "SemanticState", s::ITEM_DEFINITION),
        ("ArchitectureContracts", "Diagnostic", s::ITEM_DEFINITION),
        (
            "ArchitectureContracts",
            "ValidatedSemanticState",
            s::ITEM_DEFINITION,
        ),
        ("ArchitectureContracts", "SemanticQuery", s::PORT_DEFINITION),
        ("ArchitectureContracts", "ModelRevision", s::PORT_DEFINITION),
        (
            "ArchitectureContracts",
            "DiagnosticStream",
            s::PORT_DEFINITION,
        ),
        (
            "ArchitectureContracts",
            "SemanticAccess",
            s::INTERFACE_DEFINITION,
        ),
        (
            "LanguageArchitecture",
            "StandardLibraryManager",
            s::PART_DEFINITION,
        ),
        ("LanguageArchitecture", "SysMLEngine", s::PART_DEFINITION),
        (
            "PlatformArchitecture",
            "ProjectWorkspace",
            s::PART_DEFINITION,
        ),
        (
            "PlatformArchitecture",
            "IncrementalWorkspace",
            s::PART_DEFINITION,
        ),
        (
            "PlatformArchitecture",
            "ModelingPlatform",
            s::PART_DEFINITION,
        ),
        ("PlatformArchitecture", "QueryService", s::PART_DEFINITION),
        (
            "PlatformArchitecture",
            "ValidationService",
            s::PART_DEFINITION,
        ),
        ("PlatformArchitecture", "ViewService", s::PART_DEFINITION),
        ("PlatformArchitecture", "Repository", s::PART_DEFINITION),
        ("PlatformArchitecture", "Client", s::PART_DEFINITION),
        (
            "PlatformArchitecture",
            "ValidateRevision",
            s::ACTION_DEFINITION,
        ),
        ("PlatformArchitecture", "Serving", s::STATE_DEFINITION),
        (
            "PlatformArchitecture",
            "ImmutableRevisions",
            s::REQUIREMENT_DEFINITION,
        ),
    ] {
        let id = builder.create(class, Some(name));
        builder.member(builder.names[package], id, c::OWNING_MEMBERSHIP);
    }
    for (owner, name, definition) in [
        ("SemanticQuery", "semanticAnswer", "SemanticState"),
        ("ModelRevision", "revisionState", "SemanticState"),
        ("DiagnosticStream", "diagnostic", "Diagnostic"),
    ] {
        let child = builder.create(s::ITEM_USAGE, Some(name));
        builder.member(builder.names[owner], child, c::FEATURE_MEMBERSHIP);
        builder.enumeration(child, p::FEATURE_DIRECTION, "out");
        builder.relationship(
            c::FEATURE_TYPING,
            child,
            builder.names[definition],
            Some(p::FEATURE_TYPING_TYPED_FEATURE),
            p::FEATURE_TYPING_TYPE,
        );
    }
    for (owner, name, class, definition, composite) in [
        (
            "ProjectWorkspace",
            "kermlPublication",
            s::PART_USAGE,
            "StandardLibraryManager",
            false,
        ),
        (
            "ProjectWorkspace",
            "systemsPublication",
            s::PART_USAGE,
            "StandardLibraryManager",
            false,
        ),
        (
            "ProjectWorkspace",
            "languageQueries",
            s::PART_USAGE,
            "SysMLEngine",
            false,
        ),
        (
            "ProjectWorkspace",
            "revisionNumber",
            s::ATTRIBUTE_USAGE,
            "RevisionNumber",
            false,
        ),
        (
            "ProjectWorkspace",
            "workspaceRevision",
            s::PORT_USAGE,
            "ModelRevision",
            true,
        ),
        (
            "ProjectWorkspace",
            "workspaceQuery",
            s::PORT_USAGE,
            "SemanticQuery",
            true,
        ),
        (
            "ProjectWorkspace",
            "workspaceDiagnostics",
            s::PORT_USAGE,
            "DiagnosticStream",
            true,
        ),
        (
            "IncrementalWorkspace",
            "acceptedRevision",
            s::ATTRIBUTE_USAGE,
            "RevisionNumber",
            false,
        ),
        (
            "SemanticAccess",
            "requester",
            s::PORT_USAGE,
            "SemanticQuery",
            false,
        ),
        (
            "SemanticAccess",
            "responder",
            s::PORT_USAGE,
            "SemanticQuery",
            false,
        ),
        (
            "ModelingPlatform",
            "workspace",
            s::PART_USAGE,
            "ProjectWorkspace",
            true,
        ),
        (
            "ModelingPlatform",
            "queryService",
            s::PART_USAGE,
            "QueryService",
            true,
        ),
        (
            "ModelingPlatform",
            "validationService",
            s::PART_USAGE,
            "ValidationService",
            true,
        ),
        (
            "ModelingPlatform",
            "views",
            s::PART_USAGE,
            "ViewService",
            true,
        ),
        (
            "ModelingPlatform",
            "repository",
            s::PART_USAGE,
            "Repository",
            true,
        ),
        ("ModelingPlatform", "client", s::PART_USAGE, "Client", true),
        (
            "ModelingPlatform",
            "platformQueries",
            s::PORT_USAGE,
            "SemanticQuery",
            true,
        ),
        (
            "ModelingPlatform",
            "clientQueries",
            s::PORT_USAGE,
            "SemanticQuery",
            true,
        ),
        (
            "ModelingPlatform",
            "queryConnection",
            s::INTERFACE_USAGE,
            "SemanticAccess",
            true,
        ),
        (
            "ModelingPlatform",
            "validate",
            s::ACTION_USAGE,
            "ValidateRevision",
            true,
        ),
        (
            "ModelingPlatform",
            "operating",
            s::STATE_USAGE,
            "Serving",
            true,
        ),
        (
            "ValidateRevision",
            "workingState",
            s::ITEM_USAGE,
            "SemanticState",
            false,
        ),
        (
            "ValidateRevision",
            "checkedState",
            s::ITEM_USAGE,
            "ValidatedSemanticState",
            false,
        ),
    ] {
        let id = builder.create(class, Some(name));
        builder.member(builder.names[owner], id, c::FEATURE_MEMBERSHIP);
        builder.scalar(id, p::FEATURE_IS_COMPOSITE, Value::Boolean(composite));
        builder.relationship(
            c::FEATURE_TYPING,
            id,
            builder.names[definition],
            Some(p::FEATURE_TYPING_TYPED_FEATURE),
            p::FEATURE_TYPING_TYPE,
        );
    }
    for (name, direction) in [("workingState", "in"), ("checkedState", "out")] {
        builder.enumeration(builder.names[name], p::FEATURE_DIRECTION, direction);
    }
    for name in ["requester", "responder"] {
        builder.scalar(builder.names[name], p::FEATURE_IS_END, Value::Boolean(true));
    }
    for (specific, general) in [
        ("IncrementalWorkspace", "ProjectWorkspace"),
        ("ValidatedSemanticState", "SemanticState"),
    ] {
        builder.relationship(
            c::SUBCLASSIFICATION,
            builder.names[specific],
            builder.names[general],
            Some(p::SUBCLASSIFICATION_SUBCLASSIFIER),
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    builder.relationship(
        c::REDEFINITION,
        builder.names["acceptedRevision"],
        builder.names["revisionNumber"],
        Some(p::REDEFINITION_REDEFINING_FEATURE),
        p::REDEFINITION_REDEFINED_FEATURE,
    );
    for endpoint in ["clientQueries", "platformQueries"] {
        let end = builder.create(s::PORT_USAGE, None);
        builder.member(
            builder.names["queryConnection"],
            end,
            c::END_FEATURE_MEMBERSHIP,
        );
        builder.scalar(end, p::FEATURE_IS_END, Value::Boolean(true));
        builder.relationship(
            c::REFERENCE_SUBSETTING,
            end,
            builder.names[endpoint],
            None,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        );
    }
    for (name, kind) in [
        ("startServing", "entry"),
        ("checkRevision", "do"),
        ("stopServing", "exit"),
    ] {
        let action = builder.create(s::PERFORM_ACTION_USAGE, Some(name));
        builder.scalar(action, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        let membership = builder.member(
            builder.names["Serving"],
            action,
            s::STATE_SUBACTION_MEMBERSHIP,
        );
        builder.enumeration(membership, sp::STATE_SUBACTION_MEMBERSHIP_KIND, kind);
        if name == "checkRevision" {
            builder.relationship(
                c::FEATURE_TYPING,
                action,
                builder.names["ValidateRevision"],
                Some(p::FEATURE_TYPING_TYPED_FEATURE),
                p::FEATURE_TYPING_TYPE,
            );
        }
    }
    let subject = builder.create(s::REFERENCE_USAGE, Some("subjectWorkspace"));
    builder.member(
        builder.names["ImmutableRevisions"],
        subject,
        s::SUBJECT_MEMBERSHIP,
    );
    builder.enumeration(subject, p::FEATURE_DIRECTION, "in");
    builder.relationship(
        c::FEATURE_TYPING,
        subject,
        builder.names["ProjectWorkspace"],
        Some(p::FEATURE_TYPING_TYPED_FEATURE),
        p::FEATURE_TYPING_TYPE,
    );
    let constraint = builder.create(s::CONSTRAINT_USAGE, Some("preservesPriorState"));
    let membership = builder.member(
        builder.names["ImmutableRevisions"],
        constraint,
        s::REQUIREMENT_CONSTRAINT_MEMBERSHIP,
    );
    builder.enumeration(
        membership,
        sp::REQUIREMENT_CONSTRAINT_MEMBERSHIP_KIND,
        "requirement",
    );
    builder.finish()
}

fn declared_name(model: &ModelView, id: ElementId) -> String {
    let Some(SlotValue::Scalar(Value::String(name))) = model
        .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
        .map(|slot| slot.value())
    else {
        panic!("expected named fixture element {id}");
    };
    name.clone()
}

fn answer_names(
    model: &ModelView,
    answer: agq_kerml_semantics::QueryResult<Vec<ElementId>>,
) -> Vec<String> {
    assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
    answer
        .value
        .into_iter()
        .map(|id| declared_name(model, id))
        .collect()
}

#[test]
fn rich_vehicle_text_and_independent_changeset_have_equivalent_current_graph_answers() {
    let programmatic = programmatic_vehicle();
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        DocumentId::from_u128(920_001),
        SourceRevisionId::from_u128(920_002),
        include_str!("../tests/fixtures/vehicle.sysml"),
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let textual = lower(&syntax).strict_snapshot().unwrap();
    let summaries = [&textual, &programmatic].map(|snapshot| {
        let model = snapshot.model();
        let before: BTreeSet<_> = model.elements().map(|record| record.id()).collect();
        let queries = q(snapshot);
        assert!(queries.context().producer_closure_digest.is_none());
        let mut summary = BTreeMap::new();
        for record in model.elements().filter(|record| {
            model
                .navigation_slot(record.id(), p::ELEMENT_DECLARED_NAME)
                .is_some()
        }) {
            let name = declared_name(model, record.id());
            assert_eq!(
                named(model, &name),
                record.id(),
                "one canonical declaration per name"
            );
            summary.insert(
                format!("class/{name}"),
                vec![record.metaclass().to_string()],
            );
            if model
                .registry()
                .is_subtype(record.metaclass(), s::USAGE)
                .unwrap()
            {
                summary.insert(
                    format!("types/{name}"),
                    answer_names(model, queries.direct_feature_types(record.id())),
                );
            }
        }
        for owner in [
            "SportsCar",
            "TurboEngine",
            "MeteredFuelPort",
            "Drive",
            "Ready",
            "RoadReady",
        ] {
            // KerML's inheritance query operates on this current graph; its
            // historical name does not assert producer closure for SysML.
            let mut names = answer_names(model, queries.effective_features(named(model, owner)));
            names.sort();
            summary.insert(format!("members/{owner}"), names);
        }
        let inherited = queries.effective_features(named(model, "SportsCar"));
        assert!(inherited.value.contains(&named(model, "tank")));
        assert!(inherited.value.contains(&named(model, "supply")));
        assert!(inherited.value.contains(&named(model, "performanceEngine")));
        assert!(!inherited.value.contains(&named(model, "engine")));
        assert!(
            queries
                .effective_features(named(model, "MeteredFuelPort"))
                .value
                .contains(&named(model, "fuel")),
            "inherited port contents retain the original member identity"
        );
        assert_eq!(
            answer_names(
                model,
                queries.redefined_features(named(model, "performanceEngine"))
            ),
            ["engine"]
        );
        let endpoints = answer_names(model, queries.connector_endpoints(named(model, "fuelLine")));
        assert_eq!(
            endpoints,
            ["supply", "returnFlow"],
            "endpoint order is semantic"
        );
        summary.insert("endpoints/fuelLine".into(), endpoints);
        assert_eq!(
            model
                .elements()
                .map(|record| record.id())
                .collect::<BTreeSet<_>>(),
            before
        );
        summary
    });
    assert_eq!(
        summaries[0].keys().collect::<Vec<_>>(),
        summaries[1].keys().collect::<Vec<_>>()
    );
    for (query, textual) in &summaries[0] {
        assert_eq!(
            textual, &summaries[1][query],
            "current graph query: {query}"
        );
    }
    assert_eq!(
        summaries[0]["members/MeteredFuelPort"],
        ["flowRate", "fuel"]
    );
    assert_eq!(
        summaries[0]["members/TurboEngine"],
        ["boost", "fuelIn", "power"]
    );
    assert_eq!(summaries[0]["types/performanceEngine"], ["TurboEngine"]);
    assert_ne!(
        named(textual.model(), "tank"),
        named(programmatic.model(), "tank")
    );
    assert!(programmatic.model().elements().all(|record| matches!(
        record.origin(),
        Origin::Declared(DeclaredOrigin::Authored { source: None })
    )));
}
