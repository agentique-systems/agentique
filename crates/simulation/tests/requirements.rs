//! Requirement evaluation (C-55): a requirement's constraints calculated on
//! the modelled configuration of what satisfies it, with claims kept to
//! what was calculated.
use agq_language::{Source, Tree, parse, validate};
use agq_simulation::digest::model_digest;
use agq_simulation::requirements::{
    ConstraintKind, Evaluation, Status, Truth, evaluate, evaluate_all,
};

/// A drone whose mass is a roll-up of its airframe's and battery's, and
/// configurations of it; `extra` adds requirements and satisfy
/// relationships.
fn model(extra: &str) -> Tree {
    let text = format!(
        "package Drones {{
    private import ScalarValues::*;

    part def Airframe {{ attribute mass : Real; }}
    part def Battery {{ attribute mass : Real; }}
    part def Drone {{
        attribute mass : Real = airframe.mass + battery.mass;
        part airframe : Airframe;
        part battery : Battery;
    }}

    requirement def MassLimit {{
        doc /* The drone's mass stays within the limit. */
        subject s : Drone;
        attribute limit : Real;
        assume constraint {{ limit > 0 }}
        require constraint {{ s.mass <= limit }}
    }}

    part scout : Drone {{
        part :>> airframe {{ attribute :>> mass = 2500; }}
        part :>> battery {{ attribute :>> mass = 3000; }}
    }}
    part hauler : Drone {{
        part :>> airframe {{ attribute :>> mass = 4000; }}
        part :>> battery {{ attribute :>> mass = 3400; }}
    }}
{extra}
}}"
    );
    let tree = parse(&[Source::new("drones.sysml", text)]);
    assert_eq!(validate(&tree), [], "the model is valid");
    tree
}

/// The evaluation of the satisfy of `requirement`.
fn of<'a>(tree: &Tree, evaluations: &'a [Evaluation], requirement: &str) -> &'a Evaluation {
    let id = tree
        .find(&format!("Drones::{requirement}"))
        .unwrap_or_else(|| panic!("no {requirement}"));
    evaluations
        .iter()
        .find(|e| e.requirement == id)
        .unwrap_or_else(|| panic!("{requirement} is not evaluated"))
}

#[test]
fn a_configuration_within_the_limit_holds_and_one_over_it_is_violated() {
    let tree = model(
        "    requirement scoutMass : MassLimit { attribute :>> limit = 7000; }
    requirement haulerMass : MassLimit { attribute :>> limit = 7000; }
    satisfy scoutMass by scout;
    satisfy haulerMass by hauler;",
    );
    let all = evaluate_all(&tree);
    assert_eq!(all.len(), 2);
    let scout = of(&tree, &all, "scoutMass");
    assert_eq!(scout.status, Status::Holds, "{}", scout.reason);
    assert_eq!(scout.subject, "scout");
    assert_eq!(scout.assumptions.len(), 1);
    assert_eq!(scout.assumptions[0].truth, Truth::True);
    assert_eq!(scout.assumptions[0].text, "limit > 0");
    assert_eq!(scout.required[0].kind, ConstraintKind::Required);
    assert_eq!(scout.required[0].text, "s.mass <= limit");
    assert_eq!(
        scout.required[0].values,
        [
            ("s.mass".to_string(), "5500".to_string()),
            ("limit".to_string(), "7000".to_string())
        ],
        "the roll-up is worked out from the usage's own values"
    );
    let satisfy = scout.satisfy.expect("bound by a satisfy");
    assert_eq!(scout.digest, model_digest(&tree, satisfy));

    let hauler = of(&tree, &all, "haulerMass");
    assert_eq!(hauler.status, Status::Violated);
    assert_eq!(
        hauler.reason,
        "the required constraint `s.mass <= limit` is false (s.mass = 7400, limit = 7000)"
    );
    let text = hauler.describe();
    assert!(
        text.starts_with("violated: the required constraint `s.mass <= limit` is false"),
        "{text}"
    );
    assert!(text.contains("(subject `hauler`)"), "{text}");
    assert!(
        text.contains("assumption `limit > 0`: true (limit = 7000)"),
        "{text}"
    );
}

#[test]
fn one_definition_reused_with_different_limits_gives_different_verdicts() {
    let tree = model(
        "    requirement strict : MassLimit { attribute :>> limit = 7000; }
    requirement lenient : MassLimit { attribute :>> limit = 8000; }
    satisfy strict by hauler;
    satisfy lenient by hauler;",
    );
    let all = evaluate_all(&tree);
    assert_eq!(of(&tree, &all, "strict").status, Status::Violated);
    assert_eq!(of(&tree, &all, "lenient").status, Status::Holds);
    // Nothing was copied: both read the definition's one constraint.
    assert_eq!(
        of(&tree, &all, "strict").required[0].element,
        of(&tree, &all, "lenient").required[0].element
    );
}

#[test]
fn a_false_assumption_means_the_requirement_claims_nothing() {
    let tree = model(
        "    requirement zero : MassLimit { attribute :>> limit = 0; }
    satisfy zero by scout;",
    );
    let all = evaluate_all(&tree);
    let zero = of(&tree, &all, "zero");
    assert_eq!(zero.status, Status::AssumptionsNotMet);
    assert_eq!(
        zero.reason,
        "the assumption `limit > 0` is false (limit = 0), so the requirement claims nothing for this configuration"
    );
}

#[test]
fn an_informal_constraint_is_not_evaluable_never_a_pass() {
    let tree = model(
        "    requirement def Safe {
        subject s : Drone;
        require constraint { doc /* The drone is safe to fly. */ }
    }
    requirement scoutSafe : Safe;
    requirement def Described { subject s : Drone; doc /* Only words. */ }
    requirement words : Described;
    satisfy scoutSafe by scout;
    satisfy words by scout;",
    );
    let all = evaluate_all(&tree);
    let safe = of(&tree, &all, "scoutSafe");
    assert_eq!(safe.status, Status::NotEvaluable);
    assert_eq!(
        safe.reason,
        "the required constraint `The drone is safe to fly.`: it is informal: only its text says what it requires, so there is nothing to calculate"
    );
    let words = of(&tree, &all, "words");
    assert_eq!(words.status, Status::NotEvaluable);
    assert!(
        words.reason.contains("no required constraint"),
        "{}",
        words.reason
    );
}

#[test]
fn a_value_the_model_does_not_determine_is_named() {
    let tree = model(
        "    part stray : Drone { part :>> airframe { attribute :>> mass = 1000; } }
    requirement strayMass : MassLimit { attribute :>> limit = 7000; }
    requirement unset : MassLimit;
    satisfy strayMass by stray;
    satisfy unset by scout;",
    );
    let all = evaluate_all(&tree);
    let stray = of(&tree, &all, "strayMass");
    assert_eq!(stray.status, Status::NotEvaluable);
    assert!(
        stray.reason.contains(
            "the value of `stray.battery.mass` is not determined (the model gives it none)"
        ),
        "{}",
        stray.reason
    );
    // The limit itself is missing: the assumption cannot be calculated, so
    // whether the requirement applies is not known.
    let unset = of(&tree, &all, "unset");
    assert_eq!(unset.status, Status::NotEvaluable);
    assert!(
        unset.reason.starts_with("whether it applies is not known")
            && unset
                .reason
                .contains("the value of `unset.limit` is not determined"),
        "{}",
        unset.reason
    );
}

#[test]
fn a_violated_subrequirement_violates_its_container() {
    let tree = model(
        "    requirement def DroneLimits {
        subject d : Drone;
        requirement total : MassLimit { attribute :>> limit = 8000; }
        requirement batteryShare {
            require constraint { d.battery.mass <= 3000 }
        }
    }
    requirement scoutLimits : DroneLimits;
    requirement haulerLimits : DroneLimits;
    satisfy scoutLimits by scout;
    satisfy haulerLimits by hauler;",
    );
    let all = evaluate_all(&tree);
    let scout = of(&tree, &all, "scoutLimits");
    assert_eq!(scout.status, Status::Holds, "{}", scout.reason);
    let hauler = of(&tree, &all, "haulerLimits");
    assert_eq!(hauler.status, Status::Violated);
    assert_eq!(hauler.required.len(), 2);
    let total = hauler.required[0].subrequirement.as_ref().unwrap();
    assert_eq!(total.status, Status::Holds, "7400 is within 8000");
    assert_eq!(total.subject, "hauler", "bound to the container's subject");
    let share = &hauler.required[1];
    assert_eq!(share.kind, ConstraintKind::Subrequirement);
    assert_eq!(share.truth, Truth::False);
    assert_eq!(
        hauler.reason,
        "the subrequirement `batteryShare` is violated: the required constraint `d.battery.mass <= 3000` is false (d.battery.mass = 3400)"
    );
    assert_eq!(hauler.walk().len(), 3);
}

#[test]
fn a_usage_can_bind_its_definitions_attributes_to_the_subject() {
    // SysML 7.21.2's own pattern: the definition is written in terms of its
    // attributes, and the usage binds them to the subject.
    let tree = model(
        "    requirement def MaximumMass {
        attribute massActual : Real;
        attribute massRequired : Real;
        assume constraint { massRequired > 0 }
        require constraint { massActual <= massRequired }
    }
    requirement scoutMaximum : MaximumMass {
        subject vehicle : Drone;
        attribute :>> massActual = vehicle.mass;
        attribute :>> massRequired = 6000;
    }
    satisfy scoutMaximum by scout;",
    );
    let all = evaluate_all(&tree);
    let maximum = of(&tree, &all, "scoutMaximum");
    assert_eq!(maximum.status, Status::Holds, "{}", maximum.reason);
    assert_eq!(
        maximum.required[0].values,
        [
            ("massActual".to_string(), "5500".to_string()),
            ("massRequired".to_string(), "6000".to_string()),
            // What the bound attribute read on the way.
            ("vehicle.mass".to_string(), "5500".to_string()),
        ]
    );
}

#[test]
fn a_model_with_problems_where_it_is_read_is_not_evaluated() {
    let text = "package P {
    part def Drone { attribute mass : ScalarValues::Real = 5; }
    requirement def R { subject s : Drone; require constraint { s.weight <= 3 } }
    requirement r : R;
    part d : Drone;
    satisfy r by d;
}";
    let tree = parse(&[Source::new("p.sysml", text)]);
    let satisfy = tree.find("P").map(|p| tree[p].children()[4]).unwrap();
    let evaluation = evaluate(&tree, satisfy).unwrap();
    assert_eq!(evaluation.status, Status::NotEvaluable);
    assert!(
        evaluation.reason.contains("cannot find `weight`"),
        "{}",
        evaluation.reason
    );
}

#[test]
fn a_part_with_several_instances_is_not_read_through() {
    let text = "package P {
    private import ScalarValues::*;
    part def Rotor { attribute mass : Real = 2; }
    part def Drone {
        part rotors : Rotor[4];
        attribute mass : Real = rotors.mass;
    }
    requirement def R { subject s : Drone; require constraint { s.mass < 10 } }
    requirement r : R;
    part d : Drone;
    satisfy r by d;
}";
    let tree = parse(&[Source::new("p.sysml", text)]);
    assert_eq!(validate(&tree), []);
    let all = evaluate_all(&tree);
    assert_eq!(all[0].status, Status::NotEvaluable);
    assert!(
        all[0]
            .reason
            .contains("`d.rotors` has the multiplicity [4]"),
        "{}",
        all[0].reason
    );
}

#[test]
fn a_specialised_definition_and_a_usage_add_to_the_inherited_constraints() {
    let tree = model(
        "    requirement def LightAndSmall :> MassLimit {
        require constraint { s.battery.mass <= 3000 }
    }
    requirement scoutSmall : LightAndSmall {
        attribute :>> limit = 7000;
        require constraint airframeShare { s.airframe.mass < s.battery.mass }
    }
    satisfy scoutSmall by scout;",
    );
    let all = evaluate_all(&tree);
    let small = of(&tree, &all, "scoutSmall");
    assert_eq!(small.status, Status::Holds, "{}", small.reason);
    let texts: Vec<&str> = small.required.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "s.mass <= limit",
            "s.battery.mass <= 3000",
            "s.airframe.mass < s.battery.mass"
        ],
        "the most general definition's first, the usage's own last"
    );
    assert_eq!(small.assumptions.len(), 1, "inherited from MassLimit");
}

#[test]
fn a_subrequirement_can_bind_its_own_subject_to_a_part_of_the_containers() {
    let tree = model(
        "    requirement def BatteryLimit {
        subject b : Battery;
        require constraint { b.mass <= 3200 }
    }
    requirement def Parts {
        subject d : Drone;
        requirement batteryOk : BatteryLimit { subject b = d.battery; }
    }
    requirement scoutParts : Parts;
    requirement haulerParts : Parts;
    satisfy scoutParts by scout;
    satisfy haulerParts by hauler;",
    );
    let all = evaluate_all(&tree);
    let scout = of(&tree, &all, "scoutParts");
    assert_eq!(scout.status, Status::Holds, "{}", scout.reason);
    let sub = scout.required[0].subrequirement.as_ref().unwrap();
    assert_eq!(sub.subject, "d.battery");
    let hauler = of(&tree, &all, "haulerParts");
    assert_eq!(hauler.status, Status::Violated);
    assert!(
        hauler.reason.contains("(b.mass = 3400)"),
        "{}",
        hauler.reason
    );
}
