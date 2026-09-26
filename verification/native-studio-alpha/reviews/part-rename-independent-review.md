# Independent bounded Part rename review

Reviewed `a892ece` in the edit-performance worktree and its integration at
`3e1ecb7`. The reviewer did not author that implementation. Scope: public command
authority, source identity continuity, insertion extraction, reference stability
and effective-value equality. The follow-up correction below was explicitly
assigned to this reviewer after reporting the finding.

## Ranked findings

1. **P2 — effective graph guard omitted query-visible navigation and computation
   states.** Original `part_rename.rs:414-459` compared record slots and canonical
   association occurrences. The kernel deliberately stores association-owned
   derived navigation separately (`kernel/src/model.rs:618-623`) and separately
   stores unsuccessful computation states (`model.rs:654-662`). These participate
   in semantic identity (`kerml-semantics/src/context_digest.rs:447-458`). Equal
   record slots and occurrences therefore do not prove equal query results or
   completeness. The kernel's existing `foundation_v3.rs:480-524` independently
   demonstrates computed navigation with no corresponding record slot. This was
   a gap in the service's `effective_structure_preserved` claim; this review did
   not observe a real runtime rename exploiting it.

   **Correction:** additionally compare the complete derived-navigation key/value
   population, and the complete computation-failure key population and semantic
   payload. Incomplete reason, Invalid diagnostic and the distinction between
   those variants are retained. Missing versus computed-empty remains distinct.
   Freshly reconstructed origin, explanation and structural-search evidence is
   deliberately excluded from value equality; no old evidence is copied into the
   new model. Exact canonical element/property keys remain required, with no
   contextual-ID remapping or new name exceptions.

   Two kernel-backed counterexample tests exercise unchanged record slots and
   zero association occurrences while navigation values, navigation presence,
   failure presence, failure key, failure reason, failure variant or diagnostic
   change. Evidence-only changes remain allowed. These are non-normative kernel
   fixtures, not claims of real SysML rename acceptance.

No further P0/P1 authority or canonical-identity defect was found in this bounded
review. Real accepted-runtime rename qualification remains pending and must not
be inferred from the source proof tests.

## Identity and authority checks inspected

- `part_rename.rs:50-95,102-115`: exact current branch head, Validated predecessor,
  authored Part target, canonical source revision/node/range/kind, and an authored
  declared record are required. Standard objects and short names are refused.
- `part_rename.rs:211-380`: only one plain declared-name token is edited. Parsing
  must remain complete; every old production is matched uniquely by kind and
  transformed range, with identical count and child ownership/order. The parser
  independently checks the restored complete identity arena. Callers provide
  neither text edits nor identity maps through `RenamePart`.
- `source_identity.rs:18-43`: the source checkpoint is cloned from the resolved
  predecessor. Only the proven document bytes, revision and internally derived
  syntax arena change. Full ordinary reconstruction reuses the authenticated
  standards object; source and semantic reconstruction remain authoritative.
- `source_identity.rs:107-180`: existing authored record kinds, semantic owners
  and declared slot values remain exact, with the selected old/new declared name
  checked explicitly. The append command retains its separate owner-membership
  exception; it does not gain the rename exception.
- `source_identity.rs:182-223`: existing reference identity, name, specific target,
  resolution value and Complete status are preserved. Rename additionally
  requires the same reference count and exact declared element/occurrence ID
  populations. Source input shape and full effective graph checks remain in force.
- `source_identity.rs:225-247`: retired reservations cannot be lost or resurrected;
  rename requires them to remain exactly equal. The insertion extraction retains
  its previous subset rule and adds no caller identity authority.
- `part_rename.rs:397-406`: actual converged Complete producer closure is required.
  Working preparation does not grant validation or durable commit. Ordinary
  validation, operator authority and branch compare-and-swap remain unchanged.
- `modeling-agent/src/commands.rs:95-138`: rename still requires Read and Propose,
  prepares Working, and shows actual before/after source. It does not commit or
  confer approval through a label change.

## Qualification

The integration's real platform test now covers rejection of a referenced
definition, duplicate siblings, successful nested rename, validation/commit and a
source-only durable restart. It was not run as part of this bounded review because
the new Systems runtime was still in final audit. The kernel counterexamples
qualify the comparison guard, not the accepted-runtime workflow.

Focused qualification passed: [six rename tests](../checks/rename-query-state-tests-final.json),
[service all-target Clippy with warnings denied](../checks/rename-query-state-clippy-final.json),
[workspace formatting](../checks/rename-query-state-fmt.json) and
[diff whitespace check](../checks/rename-query-state-diff.json), all exit 0.
The original test/Clippy receipts retain a test-helper unused-Result warning/error;
the helper now asserts successful insertion of the failure fixture before building
the overlay. No production guard was weakened. A later real-model gate remains
necessary.
