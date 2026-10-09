//! Requirements with constraints (C-55): assumed and required constraints,
//! formal and informal, subrequirements and requirement usages that
//! redefine their definition's attributes; read, printed, linked and
//! validated like every other element.
use agq_language::{ElementKind, Role, Source, Tree, parse, print, validate};

fn load(text: &str) -> Tree {
    parse(&[Source::new("test.sysml", text)])
}

fn codes(tree: &Tree) -> Vec<(String, &'static str)> {
    validate(tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect()
}

fn round_trips(tree: &Tree) {
    let printed = print(tree);
    let again = parse(&printed);
    assert_eq!(
        tree.clone().without_locations(),
        again.clone().without_locations()
    );
    assert_eq!(print(&again), printed);
}

const MASS: &str = "package P {
    private import ScalarValues::*;

    part def Drone {
        attribute mass : Real;
    }

    requirement def MassLimit {
        doc /* The mass stays within the limit. */
        subject s : Drone;
        attribute limit : Real;
        assume constraint positive {
            limit > 0
        }
        require constraint {
            s.mass <= limit
        }
        require constraint safe {
            doc /* The drone is safe to fly. */
        }
        assume constraint;
        requirement airframe {
            doc /* The frame is light. */
        }
    }

    requirement group : MassLimit {
        attribute :>> limit = 7000;
        requirement light : MassLimit {
            :>> limit = 500;
        }
    }
}
";

#[test]
fn constraints_and_subrequirements_print_as_read() {
    let tree = load(MASS);
    assert_eq!(codes(&tree), []);
    assert_eq!(print(&tree)[0].text, MASS, "the text is canonical");
    round_trips(&tree);
    let def = tree.find("P::MassLimit").unwrap();
    let kinds: Vec<(ElementKind, Option<&str>, bool)> = tree[def]
        .children()
        .iter()
        .map(|c| {
            let e = &tree[*c];
            (e.kind, e.name.as_deref(), e.expression.is_some())
        })
        .collect();
    use ElementKind::*;
    assert_eq!(
        kinds,
        [
            (Doc, None, false),
            (Subject, Some("s"), false),
            (Attribute, Some("limit"), false),
            (AssumeConstraint, Some("positive"), true),
            (RequireConstraint, None, true),
            // Informal: only its doc says what is required.
            (RequireConstraint, Some("safe"), false),
            (AssumeConstraint, None, false),
            (Requirement, Some("airframe"), false),
        ]
    );
}

#[test]
fn names_in_constraints_link_to_the_subject_and_the_attributes() {
    let tree = load(MASS);
    let def = tree.find("P::MassLimit").unwrap();
    let subject = tree.find("P::MassLimit::s").unwrap();
    let limit = tree.find("P::MassLimit::limit").unwrap();
    let mass = tree.find("P::Drone::mass").unwrap();
    let required = tree[def]
        .children()
        .iter()
        .copied()
        .find(|c| tree[*c].kind == ElementKind::RequireConstraint)
        .unwrap();
    let names: Vec<Vec<_>> = tree[required]
        .references()
        .into_iter()
        .filter(|(role, _)| *role == Role::Value)
        .map(|(_, r)| r.steps.iter().map(|s| s.target).collect())
        .collect();
    assert_eq!(
        names,
        [vec![Some(subject), Some(mass)], vec![Some(limit)]],
        "`s.mass` and `limit` are linked by identity"
    );
    // A usage's redefinition names the definition's attribute.
    let redefined = tree.find("P::group::limit").unwrap();
    assert_eq!(tree[redefined].redefines[0].target(), Some(limit));
}

#[test]
fn constraints_belong_in_requirements() {
    let tree = load(
        "package P {
            part def A { attribute x = 1; require constraint { x > 0 } }
            assume constraint lonely { true }
            requirement def R { require constraint { true } }
        }",
    );
    assert_eq!(
        codes(&tree),
        [
            (
                "P::A::(require constraint)".to_string(),
                "misplaced-constraint"
            ),
            ("P::lonely".to_string(), "misplaced-constraint"),
        ]
    );
    let message = &validate(&tree)[0].message;
    assert_eq!(
        message,
        "`require constraint` belongs in a requirement def or a requirement"
    );
}

#[test]
fn a_name_a_constraint_cannot_find_is_reported() {
    let tree = load(
        "package P {
            part def Drone { attribute mass : ScalarValues::Real; }
            requirement def R {
                subject s : Drone;
                require constraint { s.weight <= limit }
            }
        }",
    );
    let diagnostics = validate(&tree);
    let found: Vec<(&str, &str)> = diagnostics
        .iter()
        .map(|d| (d.code, d.message.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("unresolved", "cannot find `weight` in the name `s.weight`"),
            ("unresolved", "cannot find the name `limit`"),
        ]
    );
}

#[test]
fn a_constraint_named_elsewhere_is_kept_verbatim_and_reported() {
    let text = "package P {
    requirement def R {
        require massLimit;
    }
}
";
    let tree = load(text);
    assert_eq!(
        codes(&tree),
        [("P::R::(unsupported)".to_string(), "unsupported")]
    );
    assert_eq!(print(&tree)[0].text, text);
}

#[test]
fn an_informal_constraint_without_a_body_reads_back() {
    let tree = load("package P { requirement def R { require constraint c; } }");
    assert_eq!(codes(&tree), []);
    let c = tree.find("P::R::c").unwrap();
    assert_eq!(tree[c].kind, ElementKind::RequireConstraint);
    assert!(tree[c].expression.is_none());
    assert!(print(&tree)[0].text.contains("require constraint c;"));
    round_trips(&tree);
}

#[test]
fn a_short_name_on_a_constraint_is_unsupported_not_a_syntax_error() {
    let text = "package P {
    requirement def R {
        require constraint <'1'> { true }
    }
}
";
    let tree = load(text);
    assert_eq!(
        codes(&tree),
        [("P::R::(unsupported)".to_string(), "unsupported")]
    );
    assert_eq!(print(&tree)[0].text, text);
}

#[test]
fn private_constraints_and_checks_are_written_and_read_back() {
    let text = "package P {
    part def D;

    requirement def R {
        subject d : D;
        private require constraint hidden {
            true
        }
    }

    verification def S {
        subject d : D;
        assert constraint opening {
            true
        }
        then private assert constraint second {
            false
        }
    }
}
";
    let tree = load(text);
    assert_eq!(codes(&tree), []);
    assert_eq!(print(&tree)[0].text, text);
    round_trips(&tree);
    for name in ["P::R::hidden", "P::S::second"] {
        let id = tree.find(name).unwrap();
        assert_eq!(tree[id].visibility, agq_language::Visibility::Private);
        assert_eq!(agq_language::writable(&tree, id), Ok(()), "{name}");
    }
}
