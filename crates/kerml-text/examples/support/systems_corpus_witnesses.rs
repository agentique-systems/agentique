//! Read-only exact-answer fixtures for scoped preflight; never publication authority.
//!
//! Structural IDs are pinned source identities from the authenticated rejected
//! corpus. They are assertions in this example, not language semantics. Queries
//! use the same construction overlay and current closure certificate as the audit.
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, EffectiveNames, StandardRole};
use agq_kerml_text::sysml::SystemsLibraryCandidate;
use agq_kernel::{
    ElementId, MetaclassId, ModelView, PropertyId,
    derived::PropertyState,
    provenance::FactKey,
    value::{SlotValue, Value},
};
use agq_sysml::{classes as sc, properties as sp};
use agq_sysml_semantics::{StandardSysmlBindings, SysmlQueries, SysmlSemanticContext};
use serde_json::json;
use std::collections::BTreeMap;

const BINARY_INTERFACE: ElementId = id(0x1c60c98bd3e4531e8b370ac695c0e9f2);
const BINARY_INTERFACES: ElementId = id(0x8ce7e6dbfd665ba19cd9a09a77092859);
const INTERFACE_ENDS: [ElementId; 2] = [
    id(0x45511382ea845db3b172c1acd4acd229),
    id(0x1e65a6e7843d5965b6452aeb545bbb03),
];
const MESSAGE: ElementId = id(0x6a26e6c7abde598ca1fbf63ad3b08bee);
const PARAMETERS: [ElementId; 2] = [
    id(0x2c8616e5d9215eaab3ec0af94bb8f701),
    id(0xa6a63b29e6ae5632b3f3ca826e32232e),
];
const HAPPENS_DURING: ElementId = id(0x51a76600246656958c538e327f6302de);
const CONNECTIONS: [(ElementId, ElementId); 2] = [
    (
        id(0x8e5fc3a742245f808c41801c3bb1b6ca),
        id(0x81be881d6fc157d18434a7267d640b15),
    ),
    (
        id(0xe76d30a12f36584d8dce409db017b746),
        id(0x48dd041c6912599a8a2a93faab66570a),
    ),
];
const FLOWS: [(ElementId, [ElementId; 2]); 3] = [
    (
        id(0x18da1d9a37e3547aac0ebf9793f724b4),
        [
            id(0x888cda79aaa55d72a654b35603802e37),
            id(0xd33b74d903dd5f678b2d6a621603eb0d),
        ],
    ),
    (
        id(0x3b9fab91096f5012841cad946c367fdf),
        [
            id(0x3dbf09d5eb9a5eca9b349091ee102a4c),
            id(0x140ef6eaf72751a089f661d454cb48c7),
        ],
    ),
    (
        id(0x5e756f07e2c756e78a4c7b5283d66543),
        [
            id(0x86de7286c60959e087ed46cf6c3f4548),
            id(0x5ec82633f2d45e14a9b1eb992bac8298),
        ],
    ),
];

const fn id(value: u128) -> ElementId {
    ElementId::from_u128(value)
}

fn is(model: &ModelView, subject: ElementId, class: MetaclassId) -> bool {
    model
        .element(subject)
        .is_some_and(|record| model.registry().is_subtype(record.metaclass(), class) == Ok(true))
}

fn references(model: &ModelView, subject: ElementId, property: PropertyId) -> Vec<ElementId> {
    model
        .navigation_slot(subject, property)
        .into_iter()
        .flat_map(|slot| slot.value().values())
        .filter_map(|value| match value {
            Value::Reference(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn declared_name(model: &ModelView, subject: ElementId) -> Option<&str> {
    match model
        .navigation_slot(subject, kp::ELEMENT_DECLARED_NAME)?
        .value()
    {
        SlotValue::Scalar(Value::String(name)) => Some(name),
        _ => None,
    }
}

fn require(findings: &mut Vec<String>, condition: bool, subject: ElementId, requirement: &str) {
    if !condition {
        findings.push(format!("{subject}: {requirement}"));
    }
}

fn owners(
    q: &SysmlQueries<'_>,
    features: &[ElementId],
    findings: &mut Vec<String>,
) -> Vec<Option<ElementId>> {
    features
        .iter()
        .map(|feature| {
            let owner = q.kerml().owning_type(*feature);
            require(
                findings,
                owner.completeness == Completeness::Complete,
                *feature,
                "canonical owning type must be Complete",
            );
            owner.value
        })
        .collect()
}

pub fn audit(
    candidate: &SystemsLibraryCandidate,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let draft = candidate.draft();
    let overlay = draft
        .semantic_candidate()
        .ok_or("witness semantic candidate missing")?;
    let certificate = draft
        .producer_closure()
        .ok_or("witness closure certificate missing")?;
    let contract = candidate.dependency_contract();
    let context = SysmlSemanticContext::for_producer_construction_overlay(
        overlay,
        candidate.accepted_kerml().complete_overlay(),
        draft.roots(),
        contract,
        StandardSysmlBindings::unbound(contract.systems_library.clone()),
    )?
    .with_producer_closure(certificate.clone())?;
    let q = SysmlQueries::new(context);
    let model = q.model();
    let original_count = model.len();
    let documents: BTreeMap<_, _> = candidate
        .documents()
        .iter()
        .map(|document| (document.document, document.path.as_str()))
        .collect();
    let includes = |basename: &str| {
        documents
            .values()
            .any(|path| path.rsplit('/').next() == Some(basename))
    };
    let source_document = |subject| {
        draft
            .source_map()
            .get(&FactKey::Element(subject))
            .and_then(|source| documents.get(&source.document))
            .copied()
    };
    let expected_definitions = usize::from(includes("SysML.sysml")) * 5
        + usize::from(includes("VerificationCases.sysml")) * 2;
    let expected_literals = usize::from(includes("SysML.sysml")) * 13
        + usize::from(includes("VerificationCases.sysml")) * 8;
    let mut findings = Vec::new();
    let mut definitions = Vec::new();
    let mut literals = Vec::new();
    let data_values = candidate
        .accepted_kerml()
        .bindings()
        .get(StandardRole::DataValues);
    for record in model
        .elements()
        .filter(|record| !draft.candidate().is_dependency_element(record.id()))
    {
        let subject = record.id();
        if is(model, subject, sc::ENUMERATION_DEFINITION) {
            let start = findings.len();
            let variation = matches!(model.property_state(subject, sp::ENUMERATION_DEFINITION_IS_VARIATION), Ok(PropertyState::Computed(slot)) if slot.value() == &SlotValue::Scalar(Value::Boolean(true)));
            require(
                &mut findings,
                variation,
                subject,
                "EnumerationDefinition variation must be canonically true",
            );
            require(
                &mut findings,
                source_document(subject).is_some(),
                subject,
                "enumeration definition must retain source identity",
            );
            definitions.push(json!({"subject":subject,"name":declared_name(model,subject),"document":source_document(subject),"is_variation":variation,"passed":start == findings.len()}));
        }
        if !is(model, subject, sc::ENUMERATION_USAGE) {
            continue;
        }
        let start = findings.len();
        let membership = q.kerml().owning_relationship(subject);
        require(
            &mut findings,
            membership.completeness == Completeness::Complete,
            subject,
            "literal owning membership must be Complete",
        );
        let owner = membership.value.and_then(|membership| {
            require(
                &mut findings,
                is(model, membership, sc::VARIANT_MEMBERSHIP),
                subject,
                "literal must retain VariantMembership",
            );
            let owner = q.kerml().owning_related_element(membership);
            require(
                &mut findings,
                owner.completeness == Completeness::Complete,
                subject,
                "literal lexical owner must be Complete",
            );
            owner.value
        });
        require(
            &mut findings,
            owner.is_some_and(|owner| is(model, owner, sc::ENUMERATION_DEFINITION)),
            subject,
            "literal owner must be EnumerationDefinition",
        );
        let typings = q
            .kerml()
            .owned_relationships_of_type(subject, kc::FEATURE_TYPING);
        let typing_targets: Vec<_> = typings
            .value
            .iter()
            .flat_map(|edge| references(model, *edge, kp::FEATURE_TYPING_TYPE))
            .collect();
        require(
            &mut findings,
            typings.completeness == Completeness::Complete
                && typings.value.len() == 1
                && typing_targets == owner.into_iter().collect::<Vec<_>>(),
            subject,
            "exactly one canonical FeatureTyping must target the owning enumeration",
        );
        for typing in &typings.value {
            require(
                &mut findings,
                references(model, *typing, kp::FEATURE_TYPING_TYPED_FEATURE) == [subject],
                subject,
                "FeatureTyping source must be the original literal",
            );
        }
        let subsets = q
            .kerml()
            .owned_relationships_of_type(subject, kc::SUBSETTING);
        let retains_data_value = subsets.completeness == Completeness::Complete
            && subsets.value.iter().any(|edge| {
                references(model, *edge, kp::SUBSETTING_SUBSETTED_FEATURE).contains(&data_values)
            });
        require(
            &mut findings,
            retains_data_value,
            subject,
            "canonical DataValue subsetting must remain",
        );
        let types = q.effective_usage_types(subject);
        let enumeration_types: Vec<_> = types
            .value()
            .iter()
            .copied()
            .filter(|target| is(model, *target, sc::ENUMERATION_DEFINITION))
            .collect();
        require(
            &mut findings,
            types.completeness() == Completeness::Complete
                && enumeration_types == owner.into_iter().collect::<Vec<_>>(),
            subject,
            "effective definition types must be Complete with exactly the owning enumeration in the enumeration projection",
        );
        let names = q.effective_names(subject);
        let name_values: Vec<_> = match names.value() {
            EffectiveNames::Determinate(names) => names.iter().cloned().collect(),
            EffectiveNames::Ambiguous { .. } => Vec::new(),
        };
        require(
            &mut findings,
            names.completeness() == Completeness::Complete
                && declared_name(model, subject)
                    .is_some_and(|name| name_values.iter().any(|actual| actual == name)),
            subject,
            "effective literal names must be Complete and retain the explicit source name",
        );
        require(
            &mut findings,
            source_document(subject).is_some(),
            subject,
            "literal must retain source identity",
        );
        literals.push(json!({"subject":subject,"document":source_document(subject),"owning_definition":owner,"variant_membership":membership.value,"feature_typings":typings.value,"owner_typing_targets":typing_targets,"typed_enumeration_definitions":enumeration_types,"usage_types":types.value(),"types_completeness":format!("{:?}",types.completeness()),"names_completeness":format!("{:?}",names.completeness()),"names":name_values,"data_value_subsetting_retained":retains_data_value,"passed":start == findings.len()}));
    }
    if definitions.len() != expected_definitions || literals.len() != expected_literals {
        findings.push(format!("scoped enumeration population: expected {expected_definitions} definitions/{expected_literals} literals, observed {}/{}",definitions.len(),literals.len()));
    }
    let mut interfaces = Vec::new();
    if includes("Interfaces.sysml") {
        for subject in [BINARY_INTERFACE, BINARY_INTERFACES] {
            let start = findings.len();
            let answer = q.effective_interface_ends(subject);
            let canonical_owners = owners(&q, answer.value(), &mut findings);
            let ports = answer
                .value()
                .iter()
                .all(|end| is(model, *end, sc::PORT_USAGE));
            require(
                &mut findings,
                answer.completeness() == Completeness::Complete
                    && answer.value() == &INTERFACE_ENDS,
                subject,
                "effective interface ends must be Complete with the exact original source and target PortUsages in order",
            );
            require(
                &mut findings,
                ports && canonical_owners == [Some(BINARY_INTERFACE); 2],
                subject,
                "inherited interface ends must retain BinaryInterface ownership and PortUsage kind",
            );
            require(
                &mut findings,
                INTERFACE_ENDS
                    .iter()
                    .all(|end| source_document(*end).is_some()),
                subject,
                "interface ends must retain source identity",
            );
            interfaces.push(json!({"subject":subject,"expected":INTERFACE_ENDS,"actual":answer.value(),"completeness":format!("{:?}",answer.completeness()),"canonical_owners":canonical_owners,"canonical_port_usages":ports,"passed":start == findings.len()}));
        }
    }
    let mut flows = Vec::new();
    let mut connections = Vec::new();
    if includes("Flows.sysml") {
        for (subject, expected_typing) in CONNECTIONS {
            let start = findings.len();
            let typings = q
                .kerml()
                .owned_relationships_of_type(subject, kc::FEATURE_TYPING);
            let targets: Vec<_> = typings
                .value
                .iter()
                .flat_map(|edge| references(model, *edge, kp::FEATURE_TYPING_TYPE))
                .collect();
            require(
                &mut findings,
                typings.completeness == Completeness::Complete
                    && typings.value == [expected_typing]
                    && targets == [HAPPENS_DURING]
                    && references(model, expected_typing, kp::FEATURE_TYPING_TYPED_FEATURE)
                        == [subject],
                subject,
                "original explicit HappensDuring FeatureTyping identity and endpoints must remain unchanged",
            );
            require(
                &mut findings,
                is(model, HAPPENS_DURING, kc::ASSOCIATION) && !is(model, HAPPENS_DURING, kc::CLASS),
                subject,
                "HappensDuring must remain a plain KerML Association",
            );
            let types = q.effective_usage_types(subject);
            require(
                &mut findings,
                types.completeness() == Completeness::Complete
                    && types.value().contains(&HAPPENS_DURING),
                subject,
                "broad effective Usage definition must be Complete and retain HappensDuring",
            );
            let mut projections = Vec::new();
            for (operation, kind, answer) in [
                (
                    "occurrence",
                    kc::CLASS,
                    q.effective_occurrence_definitions(subject),
                ),
                ("item", kc::STRUCTURE, q.effective_item_definitions(subject)),
                (
                    "part",
                    sc::PART_DEFINITION,
                    q.effective_part_definitions(subject),
                ),
                (
                    "connection",
                    kc::ASSOCIATION_STRUCTURE,
                    q.effective_connection_definitions(subject),
                ),
            ] {
                let conform = answer.value().iter().all(|target| is(model, *target, kind));
                let excludes = !answer.value().contains(&HAPPENS_DURING);
                require(
                    &mut findings,
                    answer.completeness() == Completeness::Complete && conform && excludes,
                    subject,
                    &format!(
                        "effective {operation} definition projection must be Complete, conform to its domain and exclude plain Association"
                    ),
                );
                projections.push(json!({"operation":operation,"values":answer.value(),"completeness":format!("{:?}",answer.completeness()),"targets_conform":conform,"excludes_plain_association":excludes}));
            }
            connections.push(json!({"subject":subject,"happens_during":HAPPENS_DURING,"feature_typings":typings.value,"direct_typing_targets":targets,"usage_types":types.value(),"types_completeness":format!("{:?}",types.completeness()),"projections":projections,"passed":start == findings.len()}));
        }
        for (subject, expected_ends) in FLOWS {
            let start = findings.len();
            let parameters = q.effective_parameters(subject);
            let ends = q.effective_connection_ends(subject);
            let parameter_owners = owners(&q, parameters.value(), &mut findings);
            let end_owners = owners(&q, ends.value(), &mut findings);
            require(
                &mut findings,
                parameters.completeness() == Completeness::Complete
                    && parameters.value() == &PARAMETERS,
                subject,
                "effective parameters must be Complete with the exact original Message parameters in order",
            );
            require(
                &mut findings,
                ends.completeness() == Completeness::Complete && ends.value() == &expected_ends,
                subject,
                "effective ends must be Complete with the exact original owned end identities in order",
            );
            require(
                &mut findings,
                parameter_owners == [Some(MESSAGE); 2] && end_owners == [Some(subject); 2],
                subject,
                "structural results must retain original canonical owners",
            );
            require(
                &mut findings,
                PARAMETERS
                    .iter()
                    .chain(expected_ends.iter())
                    .all(|feature| source_document(*feature).is_some()),
                subject,
                "structural features must retain source identity",
            );
            flows.push(json!({"subject":subject,"expected_parameters":PARAMETERS,"parameters":parameters.value(),"parameters_completeness":format!("{:?}",parameters.completeness()),"expected_ends":expected_ends,"ends":ends.value(),"ends_completeness":format!("{:?}",ends.completeness()),"canonical_parameter_owners":parameter_owners,"canonical_end_owners":end_owners,"passed":start == findings.len()}));
        }
    }
    let unchanged = model.len() == original_count;
    if !unchanged {
        findings.push("read-only corpus queries changed canonical element count".into());
    }
    Ok(json!({
        "schema":"agq.systems-corpus-witnesses/v1",
        "publication_authority":false,
        "source_documents":documents.values().collect::<Vec<_>>(),
        "expected":{"enumeration_definitions":expected_definitions,"enumeration_literals":expected_literals,"interface_subjects":usize::from(includes("Interfaces.sysml"))*2,"flow_subjects":usize::from(includes("Flows.sysml"))*3,"connection_subjects":usize::from(includes("Flows.sysml"))*2},
        "enumeration_definitions":definitions,
        "enumeration_literals":literals,
        "interface_ends":interfaces,
        "flows":flows,
        "connections":connections,
        "unchanged_canonical_element_count":unchanged,
        "passed":findings.is_empty(),
        "findings":findings,
    }))
}
