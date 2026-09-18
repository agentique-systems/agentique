# ADR 0008: Separate set inclusion, replacement and baseline conformance

Status: implemented for the generic kernel and offline audit, 2026-09-18.
SysML Gate 3 remains blocked by a newly exposed invalid redefinition. This ADR
supersedes ADR 0003's combined property-cycle invariant without rewriting its
historical decision or the original blocked-run evidence.

## Governing evidence

Inspected the [UML 2.5.1 PDF](https://www.omg.org/spec/UML/2.5.1/PDF) and its
[normative XMI](https://www.omg.org/spec/UML/20161101/UML.xmi), the pinned KerML 1.0
and SysML 2.0 XMI, and the supplied PDFs. Supporting acquisition identity and
constraint bodies are recorded in
[UML evidence](../../verification/sysml-language-v2-resume/uml-evidence.json).
UML is supporting authority, not a new build or generator input. No preliminary
KerML 1.1 / SysML 2.1 artifact is a runtime baseline.

UML 9.5.3 (printed pages 112–113) defines subsetting as inclusion between value
sets after duplicate elimination. Inclusion permits equality, hence reflexive
edges and mutually inclusive sets are coherent. None of the inspected Property
subsetting constraints establishes general graph acyclicity. In 9.9.17.8,
`subsetting_context_conforms` requires compatible contexts, `subsetting_rules`
requires type conformance and upper-bound narrowing, and
`subsetted_property_names` requires different names. The latter rules out a
self-subset in a UML-conformant source, independently of graph representability.
Subsetting does not require lower-bound narrowing.

Redefinition is different. UML `Property-redefined_property_inherited`,
`RedefinableElement-redefinition_context_valid` and
`isRedefinitionContextValid` require inheritance from more general contexts.
`Property::isConsistentWith` requires type compatibility, multiplicity containment
and preservation of composition. The published operation body has a repeated
lower-bound expression in its upper-bound clause; the prose and
`MultiplicityElement::compatibleWith` establish the intended bound-containment
contract. We retain the existing upper-bound check rather than translating that
expression literally. In this kernel's acyclic class hierarchy, strict context
ancestry excludes a redefinition cycle.

## Generic structural contract

The cycle graph contains only `redefines`. A self-redefinition or a cycle among
distinct properties remains `PropertyCycle`. Existence, context ancestry,
value-domain compatibility, lower/upper narrowing and composition checks remain.
Ordering and uniqueness are preserved on each exact descriptor and values are
validated against the effective replacement's shape. The former extra requirement
that redefinition monotonically preserve these flags is removed: it is not in
`Property::isConsistentWith`, and the borrowed collection API already reports the
effective descriptor and shape rather than promising its ancestor's flags.
For example, the pinned KerML `Association::associationEnd` is unordered and
redefines ordered `Type::endFeature`. Its closed descriptor graph now registers
without modifying either flag. Tests cover unordered nonunique replacements,
duplicate preservation and rejection of values using the superseded shape.
Effective property lookup still resolves replacement by identity and
checks competing inherited replacements. No language names or baseline exceptions
enter the kernel.

Subsetting checks each referenced property, context, domain and upper bound. Its
local checks do not require acyclicity, different names, equal lower bounds or
equal collection flags. Name conformance is reported by standards tooling.
Neither reflexive nor cyclic edges are normalized, removed or redirected.

`subset_closure` and `subset_contributors` use iterative visited sets in the forward
and reverse graphs. Each reachable identity contributes to the metadata frontier
at most once, even through a self-edge, cycle or diamond. Derived unions are still
uncomputed until an explicit evaluator supplies values. Such an evaluator must
deduplicate value contributions and solve cyclic inclusions by a monotone fixed
point; it must not use naive recursive expansion or claim an order that UML leaves
undefined. UML 9.5.3 only specifies ordered-union concatenation under stated
ordered-attribute conditions. Registry reachability does not supply that order.

Future incremental queries must track registry identity and inspected adjacency
sets (including absent edges), plus all positive and search dependencies of any
value evaluation. A visited set is a termination device, not evidence that a
cached result remains valid after an edge is inserted or removed.

## Baseline diagnostics are separate from registration

[Policy v1](../../standards/baseline-anomalies.json) matches specification/version,
exact artifact URI, artifact SHA-256, external property and target IDs, and exact
descriptor UUID. Target source identity must also match. A hash, identity, package
or source change does not inherit a reviewed disposition. Display names only
detect the naming violation; they never select its disposition.

The two published self-subsets remain exact:

* KerML `Kernel-Associations-A_targetType_targetAssociation-targetAssociation`,
  descriptor `c83b439d-9a37-5c2a-853f-036cbbaf5345`, XMI SHA-256
  `45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466`.
* SysML
  `Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage`,
  descriptor `7cff07a4-93d3-53ae-99fa-1ff751324480`, XMI SHA-256
  `caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`.

The complete naming check also exposed a nonreflexive same-name edge already in
the KerML Root/Core runtime: association end
`Core-Features-A_ownedRedefinition_owningFeature-owningFeature` subsets
`Core-Features-Subsetting-owningFeature` (XMI lines 5454–5462, 7098–7109).
Its context narrows from Subsetting to Redefinition, its type remains Feature and
both bounds are 0..1. This was separately inspected and given a third exact policy
entry, descriptor `232112d8-8b75-513e-a518-01d9071d1bed`, under the same pinned
KerML hash. It is not an automatic exception for other same-name edges.

Diagnostics retain severity `error`, category `normative-source-anomaly`, governing
constraint, source identity, target, message and reviewed preservation disposition.
The policy waives neither structural registration nor arbitrary UML constraints.
Unexpected naming anomalies are `unreviewed`. The report explicitly scopes this
check; it does not call either metamodel fully UML-conformant.

The audit distinguishes `structurally-representable`,
`unsupported-runtime-construct` and `invalid-generated-descriptor`, separately
from baseline diagnostics. Descriptor translation is an inspection boundary;
production generation still requires atomic registry validation. The readiness
gate rejects structural failures and unreviewed baseline diagnostics.

## Gate outcome: a separate invalid redefinition

The corrected subset invariant removes the original `PropertyCycle` failure.
It does **not** make the complete PartDefinition/PartUsage closure valid.
The first remaining rejection is:

`Systems-Flows-A_flowDefinition_definedFlow-definedFlow`
→ redefines `Systems-DefinitionAndUsage-Definition-ownedAction`.

The first property's context is KerML `Interaction`, obtained from the type of
its opposite `FlowUsage::flowDefinition`. The base is declared on SysML
`Definition`. The pinned class hierarchy does not make Interaction a subtype of
Definition. This violates the required strict redefinition-context relation,
independently of collection flags. Association-owned ends are metadata, but the required context
validation still applies. Dropping that check or guessing `definedAction` as a
replacement target would bypass this failure.

The pinned SysML XMI lines 26–38 and 95–105 encode these facts. The supplied SysML
PDF page 336, printed page 304, Figure 22 also labels `definedFlow` as redefining
`ownedAction`. No inspected authoritative correction establishes a different
target. The [new audit](../../standards/generated/sysml-2.0/structural-audit.json)
records both source entities, byte ranges, full context ancestry and the path
from PartUsage that requires this end. The [follow-up evidence](../../verification/sysml-language-v2-resume/README.md)
preserves the old audit alongside the new outcome.

The expanded KerML Association closure registers with its exact self-subset and
both metadata traversals terminate. The complete SysML gate still exits 1. It is
not justified to emit a validated `agq-sysml` runtime
or proceed to Gates 4–7. The self-subsets survive neutral import and descriptor
translation, but the complete SysML set cannot yet be published as a registry.
This limitation is explicit in the tests and coverage record. Pack 2B remains
prohibited. A resolution must preserve raw facts and descriptor identity and
establish a valid generic treatment of this independent redefinition constraint;
no blanket metadata waiver or silently corrected artifact is authorized here.
