# KerML library publication v2: participant authority conflict

Gate 0 is blocked by **KLPV2-F-001**, a conflict between the pinned abstract-syntax
XMI and the published structural semantics. This is the milestone's category F
stop condition. No library Snapshot has been accepted. The five unresolved names
and other incomplete answers remain implementation obligations; this finding does
not discharge them.

The clean starting tree was `ad6203bbac091342ed098a89ee8915595e404e57`, after
fetching `origin/main`. Work is on
`semantics/kerml-standard-library-publication-v2`.

## Exact authority

The pinned `KerML.xmi`, SHA-256
`45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466`, contains these
association-owned properties:

| XMI identity | Contract |
| --- | --- |
| `Kernel-Connectors-A_participantFeature_Association-participantFeature` | Derived, ordered, unique, `2..*`; subsets `Type::ownedFeature`. |
| `Kernel-Interactions-A_participantFeature_Interaction-participantFeature` | Non-derived, unordered, unique, `2..*`; redefines the preceding end and subsets `Behavior::involvesFeature`. |
| `Kernel-Interactions-A_participantFeature_Interaction-` | Unnamed opposite, type Interaction, default multiplicity `1..1`. |

These are not class-owned `Interaction` slots. The complete runtime correctly
retains the metadata and validates the non-derived association using occurrences.
The omission of multiplicity nodes on the opposite means `1..1` under the pinned
UML serialization defaults; it does not mean unlimited.

The supplied KerML 1.0 PDF, clause 8.3.4.4.2 (printed pages 181–183), describes
`Association::associationEnd`, a derived redefinition of `Type::endFeature`.
Clause 8.3.4.9.4 (printed page 223) lists no additional Interaction attributes,
operations or constraints. The retained XMI Association class agrees on
`associationEnd`, and the Interaction class owns no additional properties or
rules. No retained operation or constraint mentions `participantFeature`.

The [OMG issue KERML11-81](https://issues.omg.org/issues/KERML11-81) identifies the
two participant associations as obsolete remnants in the XMI and proposes their
deletion. It was reported against 1.0b4 and remains **open** at this inspection.
Its location in the 1.1 RTF tracker does not authorize adopting 1.1 semantics or
changing the pinned 1.0 artifacts. The captured issue is corroborating evidence,
not an adopted correction.

Exact source byte ranges, metadata, PDF hash, retained rule bodies and the saved
issue hash are in the [authority evidence](../verification/kerml-standard-library-publication-v2/authority-conflict.json)
and [retained authority](../verification/kerml-standard-library-publication-v2/retained-authority.json).

## Reproducible contradiction

The [synthetic regression witness](../crates/kerml-semantics/tests/participant_contract.rs)
uses seven arbitrary element IDs: two Interactions, two end Features, their two
FeatureMemberships, and one Subclassification. It uses the ordinary complete
KerML registry and kernel transactions. No library names or fallback lookup occur.

The child has no local end redeclarations. Effective-feature lookup returns the
two original parent Feature IDs with complete evidence. Strict publication
rejects the missing `participantFeature` values. If the same Features are
proposed as participant links for both Interactions, even construction preview
rejects the inverse with `actual: 2`, `required: 1..1`. The base stays empty.
The [passing witness run](../verification/kerml-standard-library-publication-v2/authority-witness-3/results.json)
records that expected rejection; it is not a successful library publication.

The exact corpus contains the same inheritance shape in `Transfer` and
`MessageTransfer`; `FlowTransfer` also inherits those ends. `TransferBefore` and
`FlowTransferBefore` add explicit redefinitions. Mapping all five to effective
ends cannot repair the raw association contract. In addition, the ancestral
participant property subsets owned features, which does not justify populating
it with inherited-only features.

There is no normative basis for treating `participantFeature` as another name
for `associationEnd`. An overlay does not remove the non-derived required
association or its opposite bound. Copying inherited Features, inventing other
participants, ignoring the bound, or deleting metadata would change canonical
semantics or publication invariants.

## Disposition

The v2 baseline again has 29,265 candidate elements, 16,747 relationships, 4,003
reference assertions, five unresolved names, 57 incomplete reference answers,
and ten lower-bound obligations. The existing semantic quality command exits 1.
Both full structural runtime gates remain separate from this failure.

The fresh baseline resolves the `incomingTransitionTrigger` member reference
completely. Its older invalid result remains in the untouched v1 report. Current
construction already suppresses an appended empty default result when a cast
has a typed result; this pre-existing work is not claimed as a v2 fix. The fresh
obligation report explicitly includes the reference even though it no longer
contributes a missing lower bound.

Gate 0 evidence, source locations, dependencies and rule-family classifications
are in the [completion matrix](../verification/kerml-standard-library-publication-v2/completion-matrix.json).
No Gate 1–16 semantic implementation or acceptance is claimed. The query rule
version remains unchanged because query answers have not changed.

Resumption requires an adopted normative correction, or an explicit project
authority decision revising the treatment of these spurious associations and
the milestone's participant acceptance requirement. An ordinary language rule
cannot reconcile the current requirements. Snapshot validation, generated
descriptors, source artifacts and inherited Feature identities remain intact.
