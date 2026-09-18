# ADR 0009: Property context is not RedefinableElement ownership

Status: investigation complete; invariant change **not accepted** pending the
authority gap below. 2026-09-18. This records a Category F blocker, not a waiver.
ADRs 0003 and 0008 and their failed gates remain historical evidence.

## Authority and reproducible facts

The baseline is published KerML 1.0 and SysML 2.0, with their existing XMI hashes.
The untouched readiness command was reproduced on fetched main `76aef699` and
returned 1. See [stage 0](../../verification/language-core-completion-v1/stage-0/results.json).
The [independent evidence](../../verification/language-core-completion-v1/stage-0/property-evidence.json)
records exact source identities, descriptor UUIDs, byte ranges, lines, ownership,
association ends, class/association ancestry, retained rules and PDF locations.

Supporting authoring authority:

* [UML 2.5.1](https://www.omg.org/spec/UML/2.5.1/PDF), clauses 9.5.3,
  9.9.17.6–8 and 9.9.18.7–8 (printed 112, 150–154; PDF 154, 192–196).
* [Final UML XMI](https://www.omg.org/spec/UML/20161101/UML.xmi), especially
  `Property-redefined_property_inherited`, `Property-isConsistentWith`,
  `Property-subsettingContext`, and
  `RedefinableElement-isRedefinitionContextValid`.
* [Resolved UML24-85 / 15525](https://issues.omg.org/issues/UML24-85).
  The resolution discusses allowing redefinition across mixed ownership in an
  inheritance hierarchy. Its motivating example has **association inheritance**.
* [Resolved UML24-86 / 15526](https://issues.omg.org/issues/spec/UML/2.4/fixed):
  `owningAssociation` contributes to `redefinitionContext` so association-owned
  ends can redefine ends of parent associations.

The final UML PDF hash is
`416b57e1933780eb48bd60fe513e031da220c28a521bdd334a366bebc78a463e`;
the final UML XMI hash is
`e8166c91f51b8a0c015a90101b83d2249d03252e9f9a500a6511654347804f21`.
These are supporting research inputs, not build or runtime dependencies.
UML is not introduced as a modeling language dependency.

[MOF 2.5.1](https://www.omg.org/spec/MOF/2.5.1/PDF), 14.1–14.4,
uses UML's class/association structures and adds CMOF constraints. These include
binary association arity; they do not supply the missing Property override.
CMOF's authoring restrictions on numeric attribute values are not a reason to
narrow the value domain of KerML LiteralInteger instances to a machine integer.

## Separate the concepts

Generic RedefinableElement validity compares redefinition contexts through
classifier ancestry. The final operation uses `allParents` and `includesAll`.
Its consistency operation defaults to false; subclasses supply consistency.
Its separate constraints require consistency and a non-leaf redefinee.

Property specializes consistency: the replacement type conforms to the base
type, its multiplicity is contained in the base multiplicity, and composition
cannot be lost. The published OCL upper-bound clause repeats lower-bound
expressions; the prose and MultiplicityElement containment contract must not be
replaced by that apparent transcription error. Ordering/uniqueness are separate
property facts, not additional monotonicity requirements in that operation.

Property ownership is either a classifier attribute or an association-owned end.
A class-owned attribute can also be an association member end. These facts are
not equivalent to Property's navigation/subsetting context: clause 9.5.3 makes
that context the owner for attributes and the types at the other ends for
association ends. `subsettingContext()` checks association membership first.
The final `type_of_opposite_end` constraint makes a binary classifier-owned
end's owner equal to the opposite type. Association ownership alone never makes
an arbitrary opposite type a subclass of the base attribute's owner.

`redefined_property_inherited` independently requires inheritance from a more
general classifier, expressed using redefinition contexts, their parents and
those parents' features. It does not authorize arbitrary replacement merely
because value types conform. UML24-85 identifies why ownership-only comparisons
are insufficient for mixed ownership; it does not abolish inheritance.

Subsetting is inclusion, with context/type/upper-bound checks. It does not
replace an inherited identity or require lower-bound narrowing. Cyclic/reflexive
subset metadata stays distinct from replacement (ADR 0008). KerML's model-level
Redefinition and Subsetting relationships are language semantics above the
kernel; they are not UML metamodel-authoring properties. Similarly, a KerML
Association class instance is not a MOF association descriptor.

## What the final publication actually contains

Neither the final PDF's complete Property operation list nor the final XMI
declares `Property::isRedefinitionContextValid`. Both retain the generic operation
and the Property inherited-feature constraint. The resolution summary proposes
an override but supplies no executable replacement body on the inspected page.
Its legacy text endpoint returned HTTP 403; that failed acquisition is recorded,
not treated as evidence about what the inaccessible resolution text contains.

Consequently we cannot truthfully describe an invented override as the operation
published in UML 2.5.1. Nor can we mechanically equate the existing kernel helper
with UML's generic operation: it already uses the opposite-end type for an
association-owned property. It is closer to binary Property subsetting context,
but is over-broadly documented as the context for both relations. It also cannot
express association inheritance (the descriptor has no superassociation field).

## Apply the candidate interpretations to the actual blocker

`Systems-Flows-A_flowDefinition_definedFlow-definedFlow` is association-owned,
derived, noncomposite, unordered, unique, 0..*, typed by FlowUsage. Its opposite
is class-owned `FlowUsage::flowDefinition`, typed by KerML Interaction.
`Systems-DefinitionAndUsage-Definition-ownedAction` is class-owned by Definition,
derived, noncomposite, ordered, unique, 0..*, typed by ActionUsage.
FlowUsage specializes ActionUsage, so the value-type and multiplicity checks
pass; the ordering difference is already allowed. There is no leaf declaration
on either property.

Neither the owning association nor the base's association has a generalization.
Their member ends and actual source owners are retained exactly. Interaction
does not specialize Definition. FlowDefinition inherits both ActionDefinition
and Interaction, but that does not make **Interaction** inherit Definition.
FlowUsage's retained `flowDefinition` description expressly permits other Kernel
Interactions, so changing its type to FlowDefinition would contradict source.
SysML PDF page 336 / printed 304, Figure 22 repeats the same redefinition.
No retained rule supplies a different property context or association ancestor.

Thus neither literal generic owner ancestry nor an ownership-independent
opposite-type ancestry test establishes this edge. The mixed-ownership issue
cannot, by itself, justify accepting it. Conversely, this does not establish
that the existing kernel's rule is correct for every legitimate Property shape.
Both the historical definitive diagnosis of an invalid source and a proposed
unconditional acceptance would exceed the authority established here.

## Decision, blocker and resumption

Keep runtime rejection and raw metadata unchanged. Classify the missing
authoritative interpretation of this exact replacement as **F**, separately
from known **E** naming anomalies and independently identifiable **C/D** numeric
and association descriptor gaps. Do not classify all future redefinitions as F.
There is no baseline allowlist entry for this edge and no name-based kernel rule.

Required external authority: the approved revised text for UML24-85 applicable
to UML 2.5.1, including its relationship to `redefined_property_inherited`, or
an OMG clarification/erratum for this exact SysML 2.0 property pair establishing
the intended inherited replacement and context. Merely asserting that mixed
ownership is allowed is insufficient: the absent association ancestry and the
Interaction/Definition relation must be explained. A correction to a future
release would not silently change the pinned 2.0 baseline.

The diagnostic pipeline may inventory the full abstract syntax and attempt
translation without publishing a runtime or altering an invariant. This is
investigation infrastructure, not passage of Gate 1 or completion of Stage 2.
Subsequent runtime/value/storage/semantic-context milestones remain unaccepted.
No replacement rule or its requested positive regression fixtures are claimed
implemented. Continue only when a coherent Property-specific contract can be
derived without discarding source facts or weakening inheritance checks.
