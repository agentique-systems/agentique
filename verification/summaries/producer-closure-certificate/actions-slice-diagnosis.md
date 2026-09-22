# Bounded Actions slice diagnosis

The lead's first eight-document slice stopped before its acceptance audit with
`Derivation(InputContextMismatch)`. Its last frontier was Structural, so the
remaining negative-query diagnostics alone do not establish a final closure
failure. No corpus execution was performed by this workstream.

The `sysml_diagnostic_sources` example maps canonical declared syntax locators
from an existing report using pinned source bytes and parsing only. It does not
load an accepted graph, lower records, resolve references, or replay producers.
It mapped all 24 distinct diagnostic subjects in the first Actions slice:
16 Actions usages, two Flows connections, five Items constraints, and one States
constraint. The three formal-negative subjects were:

| Subject | Actions.sysml byte range | Declaration |
| --- | --- | --- |
| `dcbeadeb-51ff-58e1-bc18-50a95aa2d31c` | 6086–6193 | `AcceptAction::aState` |
| `8cf3c953-ad02-5312-bf90-b8ab9dca34ee` | 6105–6189 | `aState::aTransition`, including its accept payload |
| `b7597899-b3ac-5da6-a5be-46141714bb05` | 13726–13822 | `ForLoopAction::whileLoop` action body |

Two focused regressions reproduce missing coverage without a corpus run:

- Directed Usage value: changing the existing combined variable/value fixture
  to `direction=in` reproduces `Derivation(InputContextMismatch)` (exit 101).
  This tests the non-valuation path that the undirected fixture did not cover.
- Nested state: a local ActionDefinition owns a composite StateUsage, which owns
  a composite TransitionUsage with a source Membership. A broad transition-source
  population creates a false dependency on pending positional producers.
  Pinned SysML.xmi lines 3019–3032 explicitly exclude FeatureMembership and all
  its subtypes from `TransitionUsage::sourceFeature`.

The generic excluded-subtype query regression passes (exit 0):
`cargo test -p agq-kerml-semantics --lib excluded_owned_population -- --nocapture`.
It preserves canonical order, non-feature Membership subclasses, ownership
proofs, and a relevant membership whose endpoint remains pending.

The new combined fixtures initially remain red pending the independent value
staging fix and the exact closure dependency matcher. Neither a publication nor
language-readiness claim follows from this diagnosis.
