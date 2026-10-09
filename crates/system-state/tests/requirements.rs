//! Requirement constraints through the System State (C-55): assumed and
//! required constraints and subrequirements are created and changed by the
//! same typed operations, undo restores them, and they keep their identity
//! when the project is saved and opened again.
use agq_language::{Element, ElementId, ElementKind, Literal, Parent, Reference, parse_expression};
use agq_system_state::{Actor, Change, Operation, Project, Property};

fn id(project: &Project, name: &str) -> ElementId {
    project
        .state()
        .tree()
        .find(name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn apply(project: &mut Project, operations: Vec<Operation>) -> Vec<ElementId> {
    project
        .apply(Change::new(Actor::Operator, "Edit", operations))
        .unwrap()
        .created
}

fn create(parent: ElementId, element: Element) -> Operation {
    Operation::Create {
        parent: Parent::Element(parent),
        element: Box::new(element),
    }
}

fn codes(project: &Project) -> Vec<(String, &'static str)> {
    project
        .state()
        .diagnostics()
        .iter()
        .map(|d| (project.state().tree().qualified_name(d.element), d.code))
        .collect()
}

fn text(project: &Project) -> String {
    agq_language::print(project.state().tree())[0].text.clone()
}

/// A drone with a mass, and a mass limit with an assumption, a required
/// constraint, an informal required constraint and a subrequirement.
fn mass_limit() -> (tempfile::TempDir, std::path::PathBuf, Project) {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("drone");
    let mut project = Project::create(&folder, "Drone").unwrap();
    let package = id(&project, "Drone");
    let created = apply(
        &mut project,
        vec![
            create(package, Element::named(ElementKind::PartDef, "Drone")),
            create(
                package,
                Element::named(ElementKind::RequirementDef, "MassLimit"),
            ),
        ],
    );
    let (drone, limit_def) = (created[0], created[1]);
    let mut mass = Element::named(ElementKind::Attribute, "mass");
    mass.typed_by = vec![Reference::new("ScalarValues::Real")];
    let mut subject = Element::named(ElementKind::Subject, "s");
    subject.typed_by = vec![Reference::new("Drone")];
    let mut limit = Element::named(ElementKind::Attribute, "limit");
    limit.typed_by = vec![Reference::new("ScalarValues::Real")];
    let mut assume = Element::new(ElementKind::AssumeConstraint);
    assume.expression = Some(parse_expression("limit > 0").unwrap());
    let mut require = Element::new(ElementKind::RequireConstraint);
    require.expression = Some(parse_expression("s.mass <= limit").unwrap());
    let informal = Element::named(ElementKind::RequireConstraint, "safe");
    let created = apply(
        &mut project,
        vec![
            create(drone, mass),
            create(limit_def, subject),
            create(limit_def, limit),
            create(limit_def, assume),
            create(limit_def, require),
            create(limit_def, informal),
            create(
                limit_def,
                Element::named(ElementKind::Requirement, "airframe"),
            ),
        ],
    );
    let informal = created[5];
    apply(
        &mut project,
        vec![Operation::Set {
            element: informal,
            property: Property::Doc(Some("The drone is safe to fly.".into())),
        }],
    );
    (dir, folder, project)
}

#[test]
fn constraints_are_built_by_typed_operations_and_survive_reopening() {
    let (_dir, folder, project) = mass_limit();
    assert_eq!(codes(&project), []);
    let printed = text(&project);
    for line in [
        "assume constraint {\n            limit > 0\n        }",
        "require constraint {\n            s.mass <= limit\n        }",
        "require constraint safe {\n            doc /* The drone is safe to fly. */\n        }",
        "requirement airframe;",
    ] {
        assert!(printed.contains(line), "{line}\n{printed}");
    }
    let def = id(&project, "Drone::MassLimit");
    let members = project.state().tree()[def].children().to_vec();
    drop(project);
    let project = Project::open(&folder).unwrap();
    assert_eq!(project.unmatched(), [] as [String; 0]);
    assert_eq!(
        project.state().tree()[id(&project, "Drone::MassLimit")].children(),
        members,
        "unnamed constraints keep their identity"
    );
    assert_eq!(codes(&project), []);
    assert_eq!(text(&project), printed);
}

#[test]
fn a_constraint_changes_through_set_and_undo() {
    let (_dir, _folder, mut project) = mass_limit();
    let def = id(&project, "Drone::MassLimit");
    let required = project.state().tree()[def].children()[3];
    assert_eq!(
        project.state().tree()[required].kind,
        ElementKind::RequireConstraint
    );
    let before = text(&project);
    apply(
        &mut project,
        vec![Operation::Set {
            element: required,
            property: Property::Expression(Some(parse_expression("s.mass < limit").unwrap())),
        }],
    );
    assert!(text(&project).contains("s.mass < limit"));
    assert_eq!(codes(&project), []);
    // A requirement usage gives the definition's attribute its value.
    let package = id(&project, "Drone");
    let mut usage = Element::named(ElementKind::Requirement, "light");
    usage.typed_by = vec![Reference::new("MassLimit")];
    let usage = apply(&mut project, vec![create(package, usage)])[0];
    let mut value = Element::new(ElementKind::Reference);
    value.redefines = vec![Reference::new("limit")];
    value.value = Some(Literal::Integer("7000".into()));
    apply(&mut project, vec![create(usage, value)]);
    assert!(
        text(&project).contains("requirement light : MassLimit {\n        :>> limit = 7000;"),
        "{}",
        text(&project)
    );
    assert_eq!(codes(&project), []);
    project.undo().unwrap();
    project.undo().unwrap();
    project.undo().unwrap();
    assert_eq!(text(&project), before);
}

#[test]
fn a_constraint_outside_a_requirement_is_applied_and_reported() {
    let (_dir, _folder, mut project) = mass_limit();
    let drone = id(&project, "Drone::Drone");
    let mut misplaced = Element::new(ElementKind::RequireConstraint);
    misplaced.expression = Some(parse_expression("mass > 0").unwrap());
    apply(&mut project, vec![create(drone, misplaced)]);
    assert_eq!(
        codes(&project),
        [(
            "Drone::Drone::(require constraint)".to_string(),
            "misplaced-constraint"
        )]
    );
    // A constraint's members are its doc and comments, nothing else.
    let constraint = project.state().tree()[drone].children()[1];
    let refused = project.apply(Change::new(
        Actor::Operator,
        "Edit",
        vec![create(
            constraint,
            Element::named(ElementKind::Attribute, "x"),
        )],
    ));
    assert!(refused.is_err());
}
