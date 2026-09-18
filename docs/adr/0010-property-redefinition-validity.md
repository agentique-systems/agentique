# ADR 0010: Property validity after the three OMG resolutions

Status: **Gate 0 unresolved; no runtime invariant change accepted**, 2026-09-18.
Supersedes ADR 0009's outstanding acquisition requirement and refines its authority
question. ADR 0009 and every earlier failed gate remain historical evidence.

## Evidence acquired and checked

The three official issue records were retrieved successfully, including their
complete disposition summaries. The new investigation does **not** depend on a
failed HTTP request or treat a resolved issue as open:

| Official record | Established scope |
| --- | --- |
| [UML24-85 / 15525](https://issues.omg.org/issues/UML24-85) | Resolved. Ownership-only RedefinableElement context checking is insufficient for Property. Its motivating mixed-owner case has association ancestry; its resolution calls for inheritance checking independent of Property ownership, analogous to subsetting. |
| [UML24-86 / 15526](https://issues.omg.org/issues/UML24-86) | Resolved. Association ownership contributes to redefinitionContext. |
| [UML24-96 / 15567](https://issues.omg.org/issues/UML24-96) | Resolved. Association-owned-end redefinition needs the appropriate association generalizations; the record identifies 21 examples. Endpoint-class ancestry alone cannot replace that requirement. |

[Raw pages and SHA-256 acquisition records](../../verification/language-core-completion-v2/evidence/acquisition.json)
are preserved locally. The complete fixed-issues page was also inspected.
[UML24-32](https://issues.omg.org/issues/UML24-32) and its duplicate
[UML24-34](https://issues.omg.org/issues/UML24-34) discuss symmetric end subsetting
and the links traversed by redefinition; they provide no missing hierarchy for
the SysML edge. [UML24-59](https://issues.omg.org/issues/UML24-59) separates
association-generalization changes from end-subsetting changes. None licenses
inferring an XMI generalization from a Property redefinition.

The final [UML 2.5.1 publication](https://www.omg.org/spec/UML/2.5.1) identifies
the inspected PDF and [XMI](https://www.omg.org/spec/UML/20161101/UML.xmi).
Their hashes remain those recorded in ADR 0009. We inspected PDF pages 153–155
and 191–196, including Figure 9.10, and the complete Property operation list in
XMI. `owningAssociation` does subset `redefinitionContext`, confirming UML24-86.
Two of UML24-96's association generalizations were independently located in the
final XMI. The Property context override proposed by UML24-85 is absent from both
the final operation list and the XMI declaration. The separate Property
inherited-feature constraint remains present.

These are metamodel-technology inputs only. UML is not registered as a user-model
metamodel and introduces no runtime or build dependency.

## Governing invariant and the four ownership cases

Do not accept replacement merely because the value types conform. Establish an
inheritance relationship in the relevant context, independently of names, then
check type conformance, multiplicity containment and preservation of composition.
Subsetting remains a separate inclusion relation. Do not infer association
generalization, reverse an inheritance edge, or manufacture a class slot.

For binary associations in the current descriptor architecture, define:

* `C(p)`: the owning non-association classifier for an attribute, or the opposite
  end's type for an association end. UML 9.5.3 and `subsettingContext()` establish
  this navigation/subsetting context. For a class-owned binary end, the final
  `type_of_opposite_end` constraint requires the owner and opposite type to agree.
* `A(p)`: the association of which the Property is a member, when present. For an
  association-owned end this is also its owner. A class-owned end can be a member
  of an association without being owned by it.
* `C`: strict classifier ancestry from `C(p)` to `C(q)`.
* `A`: strict association ancestry from `A(p)` to `A(q)`.

`p` is the redefining Property and `q` the redefined Property. Strict ancestry is
non-reflexive; common descendants do not establish it. This table distinguishes
what the sources establish from the unresolved reconciliation. It is deliberately
not a fabricated total validity function.

| p owner | q owner | Classifier/opposite context | Association context | Supported and unsupported arrangements | Authority and remaining question |
| --- | --- | --- | --- | --- | --- |
| Class | Class | Owning classifiers; opposite types agree for binary member ends | Membership alone does not change their ownership | `C` establishes ordinary inherited replacement; unrelated or identical owners do not. Type/bound/composition checks still apply. | Final 9.9.17.8 and 9.9.18.7–8. |
| Class | Association | Owner of p versus opposite type of q | q is owned by A(q); A(p) exists only if p is an association member | Literal owner-only comparison fails. A Property-specific comparison through `C` is suggested by UML24-85; treating it as sufficient requires reconciling the final inherited-feature constraint. No inheritance in either graph supplies no justification. | UML24-85 plus final `subsettingContext`; final `redefined_property_inherited` still needs reconciliation. |
| Association | Class | Opposite type of p versus owner of q | p is owned by A(p); q may be a member of A(q) | UML24-85's motivating case explicitly has `A`, even though literal ownership contexts differ. Whether `C` alone suffices with no `A`, and how the final inherited-feature constraint is evaluated in that case, is not specified by the final operation list. Neither `C` nor `A` fails both candidate justifications. | UML24-85, UML24-86, final 9.9.17.8. This is the actual SysML case. |
| Association | Association | Opposite types describe navigation; value-type compatibility is separate | Actual owning associations are redefinition contexts | Appropriate `A` is necessary; `C` alone is insufficient under UML24-96. Matching or unrelated association owners cannot establish strict inherited replacement. | UML24-86 and UML24-96; final owningAssociation metadata and inherited-feature constraint. |

The machine-readable [candidate matrix](../../verification/language-core-completion-v2/evidence/property-authority.json)
exposes all four ownership combinations with and without `C` and `A`. It compares
endpoint-only, association-only and either-ancestry predicates as hypotheses.
It does **not** claim these 16 rows are passing kernel validity tests. Implementing
one of those predicates for every case would invent the missing reconciliation:
endpoint-only loses UML24-96, ownership-only loses UML24-85, and an unconditional
OR admits cases without establishing the independent inherited-feature condition.

## Apply every relevant condition to the pinned acceptance case

The new offline checker reads original XMI, not the generator's interpretation
helpers. It verifies source hashes, exact UUID-v5 mapping, ownership, opposite
ends, source ranges and both inheritance graphs. Its result is
[property-authority.json](../../verification/language-core-completion-v2/evidence/property-authority.json).

| Fact | Exact pinned result |
| --- | --- |
| Redefining Property | `Systems-Flows-A_flowDefinition_definedFlow-definedFlow`, UUID `2abb2284-8e25-51ac-b486-1792cc60e1b1` |
| Redefined Property | `Systems-DefinitionAndUsage-Definition-ownedAction`, UUID `2e4efe58-2d09-5275-991e-104649d59bf3` |
| Owner of redefining Property | Association `Systems-Flows-A_flowDefinition_definedFlow` |
| Owner of redefined Property | Class `Systems-DefinitionAndUsage-Definition` |
| Redefining opposite type | KerML `Kernel-Interactions-Interaction` |
| Class-context ancestry `C` | False: Interaction does not specialize Definition |
| Association ancestry `A` | False: the owning association has no direct parents and therefore no ancestors |
| Value-type conformance | True: FlowUsage specializes ActionUsage |
| Multiplicities | Both 0..*; no multiplicity failure establishes or repairs the context |

There is a stronger reason than literal use of the generic operation: final
`Property::redefined_property_inherited` independently obtains inherited features
from parents of the redefining context. The owning association has **zero**
parents. That collection is empty, so it cannot include `Definition::ownedAction`.
Changing only `isRedefinitionContextValid` cannot make that separate condition true.
Even replacing that context with the navigation context as suggested by the
subsetting analogy does not help: Interaction has no Definition ancestor.
Even the permissive candidate `C OR A` is false for this edge.

FlowDefinition inherits both Interaction and ActionDefinition (and hence
Definition). This is a common **subclass**, not an ancestor relationship between
Interaction and Definition. The opposite property's XMI type is Interaction;
substituting FlowDefinition would change source facts and restrict the permitted
typing described by FlowUsage. Narrowing the value type from ActionUsage to
FlowUsage cannot establish inheritance on the other side of the association.

We visually inspected [SysML PDF page 336, Figure 22](../../verification/language-core-completion-v2/evidence/sysml-page-336.png),
which repeats the Interaction endpoint and `redefines ownedAction` label, and
[UML Figure 9.10](../../verification/language-core-completion-v2/evidence/uml-page-153.png).
The subsequent SysML FlowUsage clauses were also inspected. This is not an
importer typo established by disagreeing XMI and diagram facts.

Thus acceptance alternative A is unsupported. Alternative B holds **under the
published final constraints** for the precise independent reason above. This
does not establish a unique, corrected generic rule reconciling all the resolved
issues, nor authorize repairing the SysML source. That remaining authority
conflict is Category F. It is not one of the five reviewed Category E naming
anomalies and receives no new baseline disposition.

## Decision and exact resumption condition

Gate 0 is not complete. The kernel rule and generated runtime are unchanged.
No synthetic positive test is labelled normative acceptance of an invented rule.
The complete runtime commands must retain their actual failures. Gates 1–11 are
not started because the requested sequence puts this authority decision first;
their generic implementation gaps remain obligations, not reasons for this stop.

Required clarification: an authoritative Property-specific interpretation that
states how mixed-owner replacement interacts with the **separate** final
`redefined_property_inherited` constraint, and which actual context/hierarchy
validates this exact SysML edge; or an official disposition of the pinned SysML
edge with an explicit source-preserving interpretation. A statement that mixed
ownership is allowed, including UML24-85 itself, does not supply missing ancestry.
Implement the resulting generic rule and all four ownership test families only
after that conflict is resolved. No request for a one-off runtime waiver is made.
