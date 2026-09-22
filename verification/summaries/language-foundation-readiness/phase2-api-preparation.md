# SysML semantic core preparation

Status: **AGENTIQUE LANGUAGE FOUNDATION NOT YET READY FOR MODELING PLATFORM**.
Phase G has not started. This is an implementation inventory and test contract,
prepared while Systems publication remains an acceptance prerequisite. No family
completion follows from these current-graph APIs or from the textual fixture.

Authority: pinned SysML 2.0 descriptors and `standards/sysml-semantic-coverage.json`,
KerML Operational v9, SysML Operational v2. No new authority correction is proposed.

## Query boundary

`SysmlQueries` already borrows one canonical `ModelView` and composes KerML evidence.
Preserve that structure and return original IDs. All family projections must retain
filtered/rejected endpoint observations and supporting query completeness.

| Boundary | Planned contract |
| --- | --- |
| Declared | Borrow the original declared Snapshot view; do not approximate it by filtering returned record origins on a derived overlay. Derived membership/navigation can expose declared records. |
| Current graph | Evaluate the exact supplied immutable graph with existing completeness, proof, search and pending-reference contracts. A complete bounded answer does not promise future producer closure. |
| Effective | Evaluate the same current query, then merge the exact subject/requirement certificate query evidence. Missing, incompatible or insufficient certificates produce Incomplete. Preserve Invalid and underlying Incomplete results. |

Attach a certificate only after the SysML interpretation, naming adapter, standard
bindings and producer registry are fixed. Use the same context contract as the
scheduler; changing any answer-affecting input invalidates attachment. Share its
immutable `Arc`. A positive predicate witness need not request an exhaustive
population certificate, but an effective collection promises exhaustiveness.

Add a small private `require_closure` adapter that retains typed closure evidence
in the composed result, not a fabricated FactKey or a boolean-only check. Avoid
per-query proof expansion. No API may simply clear `PendingSysmlRule` entries.

## Existing and required APIs

| Family | Existing current-graph implementation | Required effective additions and closure |
| --- | --- | --- |
| Definition / Usage | `direct_usage_types`, `current_usage_types`, `direct_specializations`, `current_supertypes`, `owned_usages`, `current_effective_usages` | Definition owned/effective usages; Usage definition/nested/effective nested usages. Typing uses EffectiveTyping; both owned and inherited child membership populations use EffectiveMembership. EffectiveOwnership concerns parent absence/exact owner domain, not child population. Add explicit current aliases for ambiguous names. |
| Specialization / subsetting / redefinition | `subsetted_features`, `redefined_features`, `all_redefined_features` | Preserve current variants; effective exhaustive relationship sets require explicit relationship requirements, conservatively EffectiveTyping while its effect mapping covers every queried relation. No inherited record allocation. |
| Attribute / Item / Part | `current_attribute_definitions`, `current_item_definitions`, `current_part_definitions` | Effective definition queries use EffectiveTyping. Owned/effective attributes/items/parts are evidence-preserving selectByKind projections of usage membership. Subitems/subparts retain canonical subsetting targets and composite flags; gate all effects actually read. |
| Port | `current_port_definitions`; inherited members available through KerML | Effective port definitions and owned/effective ports. Verify inherited port member identity, conjugation and typed domain; do not replace a conjugated port with a copied definition. |
| Connection / Interface | `current_connection_related_features` accepts ConnectorAsUsage | Add current/effective connection and interface definitions, ends and related features. Reuse KerML connector-end and related-feature queries. The existing ValueContext requirement conservatively covers connector, result-structure, featuring and typing effects; use it with the actual query dependencies. ConnectorAsUsage remains the input domain where the formal query requires it. |
| Occurrence / Action / State | Family producers exist; generic membership and type queries are reusable | Add parameters, subactions, entry/do/exit, transition actions and AcceptAction accepter/message structure. StateSubactionMembership.kind selects roles; transition membership roles stay distinct. Use effective membership plus the specific result/connector/typing dependencies read. No execution semantics. |
| Requirement / Constraint / Case | Generic type/membership foundation; specialized role queries absent | Add requirement/constraint/case, analysis/verification projections, subjects, actors, objectives and return parameters. Read canonical SubjectMembership, ActorMembership, ObjectiveMembership and ReturnParameterMembership with completeness/evidence. Do not substitute a class-name filter for membership roles. |
| View / Viewpoint / Rendering / Metadata | Generic projections available; specialized facade absent | Add owned/effective family usage and definition projections, using Operational v2 standard targets and the same typed domain/membership gates. No graphical or execution layer. |
| Names | `effective_names` and `effective_qualified_name` currently inspect current graph | Introduce `current_names` and `current_qualified_name`; make effective counterparts certificate-backed. EffectiveNaming covers names; sibling collision/uniqueness and owner-chain absence additionally require membership/ownership closure. Preserve unsupported inherited full-name selection diagnostics. |

`effective_usage_types` and `effective_usages` currently call
`pending_implications`, which unconditionally inserts ProducerClosure and scans
unrelated mayTimeVary, individual, portion and specialized-family obligations.
Replace that blanket mechanism with the query's explicit closure requirement.
Keep real unsupported variation or narrowed-domain findings when they affect the
answer. `current_part_definitions` has a required-domain pending diagnostic; a
closed empty required domain becomes a supported validation/domain failure, not
a silently successful empty type answer.

The formal domain is authoritative: Attribute typing selects DataType; Item
selects Structure from its Class domain; Part selects PartDefinition; Port uses
PortDefinition. The formal ownedInterface selectByKind domain must be inspected
directly instead of assuming it equals the property label. Existing out-of-domain
and general-ancestor pruning tests are regression requirements.

## Focused acceptance tests to implement after publication

1. On one graph, current query is Complete while effective query without a
   certificate is Incomplete; attach the scheduler's real certificate and retain
   the same IDs with Complete evidence. Reject stale graph, profile and registry.
2. Delayed relevant producers keep the effective answer incomplete; a completed
   positive predicate remains available without an unrelated negative gate.
3. Definition/Usage type and member matrices cover every supported family and
   ordinary valid KerML endpoints. Filtering must preserve rejected evidence.
4. Inherited diamond members retain original IDs; redefining a member suppresses
   only the correct inherited population; no copied Usage records are created.
5. Port inheritance preserves its original member IDs. Connection/interface end
   roles, related features and actual endpoints are distinct and fully resolved.
6. State entry/do/exit and transition/accepter/message queries follow membership
   roles; action parameter direction and return membership remain explicit.
7. Requirement/case subjects, actors, objectives and returns preserve identities
   and ordering where the formal rule uses first/ordered membership. Empty
   optional roles need closure before an effective negative conclusion.
8. View/Viewpoint/Rendering/Metadata use Operational v2 targets. Existing explicit
   Published/v1 selection remains available and cannot inherit v2 answers.
9. The rich Vehicle/SportsCar textual fixture and an independently built kernel
   ChangeSet equivalent produce equal semantic projections, not equal syntax IDs.
   Run both against shared accepted KerML and Systems dependencies and verify
   dependency mismatch rejection, stable standard IDs and qualified names.

The rich authored source and focused parsing/lowering test are prepared separately
by the Actions workstream. They establish useful fixture structure, not accepted
effective semantics. The final language-readiness gate still requires the accepted
Systems publication and all requested family/vertical results above.
