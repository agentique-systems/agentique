# Bounded real-corpus gate plan

The combined 16-document candidate covers every prior diagnostic subject:
47 local Systems subjects and three targets in the shared accepted KerML v9
dependency. No prior subject requires a seventeenth Systems document.
This is a verified attribution and a proposed run plan, not semantic acceptance.

| Gate | Candidate documents | Reason |
| --- | --- | --- |
| H1 enumeration | Actions, Attributes, Calculations, Cases, Connections, Constraints, Flows, Interfaces, Items, Metadata, Parts, Ports, Requirements, States, SysML, VerificationCases | All seven actual enum definitions occur in SysML (five) and VerificationCases (two). The required States/Requirements documents and their source dependencies are included. SysML's metadata definitions additionally require the Metadata semantic base. |
| H2 Flow/Interface | Actions, Calculations, Connections, Constraints, Flows, Interfaces, Items, Parts, Ports, States | The earlier eight-document Actions foundation supplies cyclic semantic bases. Interfaces adds a private calculation definition, excludingOnce, which requires Calculations despite having no explicit import of it. |
| H3 result cycles | Actions, Connections, Constraints, Flows, Items, Parts, Ports, States | All 21 old result-cycle subjects belong to Actions (eight), Flows (nine), or States (four). This is the exact previously closed eight-document slice. |
| H4 medium | Actions, Attributes, Calculations, Connections, Constraints, Flows, Interfaces, Items, Parts, Ports, Requirements, States, Views | Exact prior 13-document population; 695 mandatory source references. |

H1 already contains H2 and H3. One fresh combined H1–H3 producer closure can
therefore run the strict effective audit over its entire local population and
report each root family's subjects. That run must establish all three requested
gate outcomes, including exact enumeration types/names, interface ends, the
ConnectionUsage authority disposition, and effective parameters/returns for the
prior cycle subjects. In particular, the old KQ_END_CYCLE findings were surfaced
by effective parameters; an end-only check would miss that operation. The
separate exact 13-document medium run follows only after this combined gate
passes. Neither slice may issue Systems publication artifacts.

This combined execution does not claim that the smaller H2/H3 populations were
independently run or that arbitrary document subsets are semantically closed.
It shares the accepted standards and preserves the full effective audit, while
avoiding redundant closures over overlapping documents. The full 21-document
fresh scheduler and authenticated strict finalizer remain separate requirements.

All 50 prior diagnostic subjects map as follows:

| Canonical source document | Subjects |
| --- | ---: |
| SysML.sysml | 13 |
| Flows.sysml | 11 |
| VerificationCases.sysml | 10 |
| Actions.sysml | 8 |
| States.sysml | 4 |
| Interfaces.sysml | 1 |
| Accepted KerML Links.kerml | 2 |
| Accepted KerML Occurrences.kerml | 1 |

The accepted dependency subjects are BinaryLink's original source and target
Features, and HappensDuring. Their graphs are not copied or regenerated.
The 21 result-cycle subjects break down into ControlAction's two unnamed
Features, AcceptAction's two, TransitionAction's four, StateTransitionAction's
four, and messages/flows/successionFlows with their six source/target Usages.
Exact IDs and every canonical ownership edge are retained in
`verification/fixtures/final-audit-semantic-closure/slices.json`.

Reproduce the inventory with `verification/scripts/plan_final_audit_slices.py`,
passing the retained `c42d679d...zip` as `--frontier`, the trusted v9 publication
ZIP as `--kerml-cache`, and that fixture path as `--output`. The script verifies
the archive and decoded graph hashes against the historical authentication
record; it checks the accepted dependency graph against its independent receipt
and the classifier's pin. Package identities are recomputed from exact KPAR
source hashes, source document identity, byte range, role and ordinal before
matching ownership paths. No semantic producer or query evaluator runs.

The source dependency scan reads `standards/artifacts/Systems-Library.kpar`,
verified through `standards/normative/sysml-2.0/library-set.json`. The older
April extracted library directory has different bytes and is not used. Lexical
qualified-name references provide a lower bound; the explicit semantic
supplements above and the previously passed slice supply the reviewed candidate
sets. Only actual new closure and query results can certify their success.

Historical reference evidence is retained in
`verification/summaries/language-stability-bridge/publication-commands.json`:
`actions-owned-cross-subsetting` closed all producer pairs with 534 Complete
references. The prior medium had 695 Complete references. Their old producer
pair/requirement counts and checkpoints are not reused after the new registry.

Actual inventory commands and exits are appended to
`verification/summaries/enumeration-closure/commands.json`; raw outputs remain
under ignored `verification/generated/enumeration-closure/`.
