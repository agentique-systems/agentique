//! Validation rules on small models. Each test states the problems it expects
//! as (element, code) pairs, in document order.
use agq_language::{Source, parse, validate};

fn problems(text: &str) -> Vec<(String, &'static str)> {
    let tree = parse(&[Source::new("test.sysml", text)]);
    validate(&tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect()
}

fn expect(text: &str, expected: &[(&str, &str)]) {
    let actual = problems(text);
    let actual: Vec<(&str, &str)> = actual.iter().map(|(e, c)| (e.as_str(), *c)).collect();
    assert_eq!(actual, expected, "\n{text}");
}

#[test]
fn imports_and_visibility() {
    expect(
        "package Lib { part def Engine; private part def Secret; }
         package A { private import Lib::*; part e : Engine; }
         package B { import Lib::Engine; part e : Engine; }
         package C { part e : A::Engine; }
         package D { part s : Lib::Secret; }
         package E { public import Lib::*; }
         package F { part e : E::Engine; part n : Lib::Nothing; }",
        &[
            ("C::e", "unresolved"),
            ("D::s", "unresolved"),
            ("F::n", "unresolved"),
        ],
    );
}

#[test]
fn nested_packages_and_private_members_inside() {
    expect(
        "package P {
             package Q { private part def Hidden; part def X; part h : Hidden; }
             part x : Q::X;
         }",
        &[],
    );
}

#[test]
fn two_imports_of_the_same_name_are_ambiguous() {
    expect(
        "package L1 { part def X; }
         package L2 { part def X; }
         package U { import L1::*; import L2::*; part x : X; }
         package V { import L1::*; import L1::X; part x : X; }",
        &[("U::x", "ambiguous")],
    );
}

#[test]
fn inherited_features_are_found_by_lookup_and_ports_must_fit() {
    expect(
        "package P {
             item def Msg;
             port def Pt { in item msg : Msg; }
             part def Base { port p : Pt; port q : ~Pt; }
             part def Derived :> Base;
             part def Sys {
                 part a : Derived;
                 part b : Derived;
                 connection ok connect a.p to b.q;
                 connection bad connect a.p to b.p;
                 connection missing connect a.p to b.r;
             }
         }",
        &[
            ("P::Sys::bad", "incompatible-ends"),
            ("P::Sys::missing", "unresolved"),
        ],
    );
}

#[test]
fn port_directions_must_mirror_across_different_port_defs() {
    expect(
        "package P {
             item def Msg;
             port def Sender { out item msg : Msg; }
             port def Receiver { in item msg : Msg; }
             port def Other { in item other : Msg; }
             interface def Link { end port a : Sender; end port b : Receiver; }
             interface def Broken { end port a : Sender; end port b : Other; }
         }",
        &[("P::Broken", "incompatible-ends")],
    );
}

#[test]
fn redefinition_must_target_an_inherited_feature() {
    expect(
        "package P {
             private import ScalarValues::*;
             part def A { attribute x : Natural; }
             part def B :> A { attribute :>> x = 3; attribute :>> y = 1; }
             part def C :> A { attribute x : Natural; }
             part a : A { attribute :>> x = 4; }
         }",
        &[
            ("P::B::y", "redefines-unknown"),
            ("P::C::x", "duplicate-name"),
        ],
    );
}

#[test]
fn redefinition_through_a_diamond() {
    expect(
        "package P {
             part def Top { attribute v; }
             part def L :> Top { attribute :>> v; }
             part def R :> Top;
             part def Bottom :> L, R { attribute :>> v; }
             part def L2 { attribute w; }
             part def R2 { attribute w; }
             part def B2 :> L2, R2 { attribute :>> w; }
         }",
        &[("P::B2::w", "ambiguous")],
    );
}

#[test]
fn usages_must_be_typed_by_the_right_kind_of_definition() {
    expect(
        "package P {
             part def Pd; port def Qd; item def Id;
             part a : Qd;
             port b : Pd;
             part c : ~Pd;
             part d : a;
             item e : Pd;
             part f : Id;
             attribute g : Pd;
         }",
        &[
            ("P::a", "wrong-type"),
            ("P::b", "wrong-type"),
            ("P::c", "wrong-type"),
            ("P::d", "wrong-type"),
            ("P::f", "wrong-type"),
            ("P::g", "wrong-type"),
        ],
    );
}

#[test]
fn definitions_specialise_compatible_definitions_without_cycles() {
    expect(
        "package P {
             port def Q; item def I;
             part def A :> Q;
             part def B :> I;
             part def C :> D;
             part def D :> C;
         }",
        &[
            ("P::A", "wrong-kind"),
            ("P::C", "specialization-cycle"),
            ("P::D", "specialization-cycle"),
        ],
    );
}

#[test]
fn names_multiplicities_and_values() {
    expect(
        "package P {
             private import ScalarValues::*;
             part def A;
             part def A;
             part def B;
             part p : B[3..1];
             part q : B[0..*];
             attribute n : Natural = -1;
             attribute s : String = \"ok\";
             attribute r : Real = 2;
             attribute b : Boolean = 1;
         }",
        &[
            ("P::A", "duplicate-name"),
            ("P::p", "bad-multiplicity"),
            ("P::n", "wrong-value"),
            ("P::b", "wrong-value"),
        ],
    );
}

#[test]
fn a_package_named_like_the_library_is_reported_and_shadows_it() {
    // The document's own `ScalarValues` is found before the library's.
    expect(
        "package ScalarValues { attribute def Text; }
         package P { attribute a : ScalarValues::String; }",
        &[("ScalarValues", "duplicate-name"), ("P::a", "unresolved")],
    );
}

#[test]
fn requirements_are_satisfied_by_features_of_the_subject_type() {
    expect(
        "package P {
             part def Server; part def FastServer :> Server; part def Client;
             requirement def Fast { subject s : Server; }
             part def Sys { part server : FastServer; part client : Client; }
             part sys : Sys;
             requirement fast : Fast;
             satisfy fast by sys.server;
             satisfy fast by sys.client;
             satisfy Fast by sys.server;
             part def Misplaced { subject s : Server; }
         }",
        &[
            ("P::(satisfy fast)", "wrong-subject"),
            ("P::(satisfy Fast)", "wrong-kind"),
            ("P::Misplaced::s", "misplaced-subject"),
        ],
    );
}

#[test]
fn interfaces_connect_ports_and_ends_belong_in_connection_defs() {
    expect(
        "package P {
             part def A;
             part def S { part a : A; part b : A; interface i connect a to b; connect a to b; }
             part def E { end part x : A; }
             connection def C { end part x : A; end part y : A; }
             part def T { part a : A; part b : A; connection c : C connect a to b; }
         }",
        &[("P::S::i", "wrong-kind"), ("P::E::x", "misplaced-end")],
    );
}

#[test]
fn a_part_def_cannot_require_itself() {
    expect(
        "package P {
             part def Node { part next : Node[0..1]; }
             part def Wheel { part hub : Hub; }
             part def Hub { part wheel : Wheel; }
             part def Tree { part children : Tree[*]; }
         }",
        &[
            ("P::Wheel", "composition-cycle"),
            ("P::Hub", "composition-cycle"),
        ],
    );
}

#[test]
fn import_cycles_end() {
    expect(
        "package A { public import A::*; part x : Nope; }
         package B { public import C::*; part y : Nope; }
         package C { public import B::*; part def Here; }
         package D { import B::*; part z : Here; }",
        // D sees `Here` because B re-exports C through its public import.
        &[("A::x", "unresolved"), ("B::y", "unresolved")],
    );
}

#[test]
fn a_top_level_import_serves_its_own_document_only() {
    let tree = parse(&[
        Source::new("lib.sysml", "package Lib { part def Engine; }"),
        Source::new(
            "car.sysml",
            "import Lib::*;
package Car { part engine : Engine; }",
        ),
        Source::new("bike.sysml", "package Bike { part engine : Engine; }"),
    ]);
    let problems: Vec<(String, &str)> = validate(&tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect();
    assert_eq!(problems, [("Bike::engine".to_string(), "unresolved")]);
}

#[test]
fn protected_members_are_visible_to_specialisations_only() {
    expect(
        "package P {
             part def Base { protected attribute q; protected part def Inner; }
             part def Derived :> Base { attribute r :> q; part i : Inner; }
             part def User { attribute u :> Derived::q; part j : Base::Inner; }
         }",
        &[("P::User::u", "unresolved"), ("P::User::j", "unresolved")],
    );
}

#[test]
fn specialising_oneself_is_a_cycle() {
    expect(
        "package P { part def A :> A; part def B { attribute x :> x; } }",
        &[
            ("P::A", "specialization-cycle"),
            ("P::B::x", "specialization-cycle"),
        ],
    );
}

#[test]
fn ends_and_subjects_redefine_implicitly() {
    expect(
        "package P {
             item def M;
             port def Pt { out item m : M; }
             interface def I { end port a : Pt; end port b : ~Pt; }
             interface def J :> I { end port c : Pt; end port d : ~Pt; }
             interface def K :> I { end port a : Pt; end port b : ~Pt; }
             part def S { port p : Pt; port q : ~Pt; }
             part def Sys {
                 part s1 : S;
                 part s2 : S;
                 interface j : J connect s1.p to s2.q;
                 interface k : K connect s1.p to s2.q;
                 interface bad : J connect s2.q to s1.p;
             }
             part def Special :> S;
             requirement def R { subject s : S; }
             requirement r : R { subject t : Special; }
             requirement r2 : R { subject s : S; }
         }",
        // One problem per end that does not fit.
        &[
            ("P::Sys::bad", "incompatible-ends"),
            ("P::Sys::bad", "incompatible-ends"),
        ],
    );
}

#[test]
fn a_sender_may_send_a_specialisation_of_what_is_received() {
    expect(
        "package P {
             item def M; item def M2 :> M;
             port def SendsM2 { out item m : M2; }
             port def SendsM { out item m : M; }
             port def TakesM { in item m : M; }
             port def TakesM2 { in item m : M2; }
             part def A { port out2 : SendsM2; port out1 : SendsM; port in1 : TakesM; port in2 : TakesM2; }
             part def Sys {
                 part a : A; part b : A;
                 connect a.out2 to b.in1;
                 connection narrowing connect a.out1 to b.in2;
             }
         }",
        &[("P::Sys::narrowing", "incompatible-ends")],
    );
}

#[test]
fn a_port_may_pass_items_on_to_an_inner_part() {
    expect(
        "package P {
             item def M;
             port def Takes { in item m : M; }
             part def Inner { port p : Takes; port q : ~Takes; }
             part def Outer {
                 port p : Takes;
                 part inner : Inner;
                 connection passOn connect p to inner.p;
                 connection reversed connect p to inner.q;
             }
         }",
        &[("P::Outer::reversed", "incompatible-ends")],
    );
}

#[test]
fn delegation_is_decided_by_which_part_contains_which() {
    expect(
        "package P {
             item def M;
             port def Takes { in item m : M; }
             part def Inner { port i : Takes; }
             part def Sub { port i : Takes; part inner : Inner; }
             part def Top { part sub : Sub; connection passOn connect sub.i to sub.inner.i; }
             port g : Takes;
             part def Other { part inner : Inner; connection fromPackage connect g to inner.i; }
         }",
        &[("P::Other::fromPackage", "incompatible-ends")],
    );
}

#[test]
fn many_mutually_importing_packages_resolve_quickly() {
    let n = 20;
    let mut text = String::new();
    for i in 0..n {
        text.push_str(&format!("package P{i} {{\n"));
        for j in (0..n).filter(|j| *j != i) {
            text.push_str(&format!("    public import P{j}::*;\n"));
        }
        let next = (i + 1) % n;
        text.push_str(&format!(
            "    part def D{i};\n    part x{i} : D{next};\n    part y{i} : Missing;\n}}\n"
        ));
    }
    let start = std::time::Instant::now();
    let tree = parse(&[Source::new("many.sysml", text)]);
    let problems = validate(&tree);
    let elapsed = start.elapsed();
    assert_eq!(problems.len(), n, "only the `Missing` types are unresolved");
    assert!(elapsed < std::time::Duration::from_secs(1), "{elapsed:?}");
}
