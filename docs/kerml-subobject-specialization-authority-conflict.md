# KLCV6-F-001 — Conflicting required subobject specialization target

The expanded formal-constraint audit independently reproduces a published
authority conflict outside KERML11-81/140/76/68. Its corresponding OMG issue is
[KERML11-205](https://issues.omg.org/issues/KERML11-205), currently open.

KerML 1.0's `checkFeatureSubobjectSpecialization` names
`Objects::Object::subobjects` in its normative prose and semantic explanation,
but `Occurrence::Occurrence::suboccurrences` in its formal expression. The pinned
PDF (physical page 194, printed page 168) and pinned metamodel XMI agree on that
disagreement. The captured preliminary 1.1 Beta 2 still contains it. There is no
`Occurrence` package in the exact library set: its name is `Occurrences`.

This is not a generic incomplete lookup or an unimplemented inheritance rule.
The pinned `Objects::Object::subobjects` declaration itself is an independently
reproducible witness. It is explicitly composite, owned by the Structure Object,
and explicitly typed by Object. Thus the semantic antecedent applies without
requiring any implied relationship. Under the prose, it satisfies the required
specialization reflexively: Type::allSupertypes includes the Type itself. Under
the formal target, `specializesFromLibrary` fails its required membership lookup.

| Interpretation | Required target | Result on the witness |
| --- | --- | --- |
| Published formal target requirement | `Occurrence::Occurrence::suboccurrences` | Missing target; requirement unsatisfied under the documented antecedent |
| Published prose and semantic example | `Objects::Object::subobjects` | Reflexively satisfied |
| Repair only the package spelling | `Occurrences::Occurrence::suboccurrences` | A different canonical Feature |

Choosing the subobject target, choosing the broader suboccurrence target, adding
a nonexistent namespace, or omitting the requirement changes the required
canonical relationships or validation disposition. This therefore requires an
explicit semantic authority decision, beyond merely implementing missing order
or inheritance. The exact OCL is also retained with its other typing/spelling
differences; the reproduction evaluates the documented antecedent and the target
requirement separately, and does not claim to execute the literal OCL expression.

The official pilot's FeatureAdapter and ImplicitGeneralizationMap select the
prose's subobject target. Current implied XMI corroborates the explicit witness.
These are operational evidence, not permission to expand the newly authorized
KERML11-68 correction silently. KERML11-205 also reports the separate missing
`subperformance` target, and KERML11-207 reports related Step target spellings;
those are recorded as corroborating inventory findings, not additional hidden
v4 corrections.

The [machine-readable packet](../verification/kerml-semantic-closure-v6/subobject-authority-conflict.json)
contains exact formal XML and PDF clauses, archive and source hashes/range,
reference identities, applicable flags/types, all five profile witnesses and
the materially different dispositions. The independent Python verifier uses
ZIP/XML facts and formal operations; it does not call Agentique inference helpers.
The arbitrary-name Rust witness obtains complete answers for all target searches,
including the empty literal search, in all five profiles.

```text
python -B -X utf8 verification/kerml-semantic-closure-v6/verify-subobject-authority.py --check
cargo test --locked --offline -p agq-kerml-semantics --test subobject_authority -- --nocapture
```

No correction for this conflict has been applied. Strict publication, accepted
bindings and the accepted library facade remain withheld. Ordinary structural
implementation work remains mandatory after the authority decision; this stop
does not waive or disguise it.
