# Source-command identity review

The bounded RenameElement implementation was withdrawn before integration. The
current production frontend does not preserve the selected declaration's syntax
identity when its name token is edited. An apparently successful source rename
would therefore delete and recreate the canonical engineering object. The existing
unsupported-command result remains in place; no Rename UI or platform capability
was added.

## Concrete mechanism and retained regressions

`production::Document::edit` retains IDs only for nodes outside the edit range.
Overlapping nodes receive fresh IDs. This includes the renamed declaration and
containing declarations. `library::construction::authored_id` derives authored
ElementIds from the SyntaxNodeId and construction role, while
`source_inputs::prepare_identity_history` retires source identities absent from
the new syntax arena. This is canonical identity churn, not merely a view label.

Two focused modeling-agent regressions now make the current behavior explicit:

- `production_name_edits_replace_selected_and_ancestor_identity`: a name-only
  edit retires the selected PartUsage and containing Package syntax IDs while
  retaining a disjoint sibling.
- `nested_insertion_retains_disjoint_children_but_replaces_owner_identity`:
  the actual existing `nested_part_edit` mapping preserves an unchanged child,
  but assigns a fresh ID to its PartDefinition owner because the insertion lies
  inside that owner's range.

The second observation limits the existing CreatePartUsage command too. Its
postcondition checks the new declaration and selected definition where provided;
it does not prove that the original owner retained its canonical ID. The real
runtime journey must check this instead of relying on the fixture's stable IDs.
These parser tests do not substitute for that semantic reconstruction gate.

## Why checkpoint restore is not an existing rename operation

`Document::restore_identities` accepts an identity checkpoint only after verifying
production, ranges and child shape against a parsed arena; its caller must already
authenticate the source. The durable source checkpoint similarly restores a
previously issued revision. Neither API proves that identities may be transferred
from one edited declaration to another.

`ProjectChange::Edit` carries only a document and text edit. There is no reviewed
identity-transfer carrier in the normal ModelingService preparation path. Rewriting
source checkpoint IDs and hashes outside that path would introduce an unreviewed
identity authority surface. The exploratory command did not do this.

## Required contract

A command-layer implementation could be sound only if it issues and consumes an
explicit proof of identity reconciliation. It must be narrower than accepting an
arbitrary caller-supplied checkpoint:

1. Bind intent to the exact project, branch head, document/source revision and
   selected canonical declaration, with its source node and range verified from
   the current model. Capture the old complete production arena independently.
2. Support one reviewed edit class at a time. For nested Part insertion, existing
   declaration headers, kinds and ownership must remain unchanged; only the
   selected owner's body grows. For rename, exactly one verified name token may
   change. Reference updates and shadowing checks are separate proof obligations.
3. Parse the proposed bytes under the same pinned profile. Establish an unambiguous
   one-to-one mapping for every retained old node using production, transformed
   range, old/new child relationships and unchanged source outside the permitted
   edit. Reject ambiguity, omissions, identity reuse, owner changes and changed
   kinds. New subtree nodes receive new IDs. A matching name alone is insufficient.
4. Have the service derive or independently verify that mapping; a frontend or
   agent cannot assert it. Carry the verified mapping through source preparation
   without an intermediate pass retiring the retained IDs. Preserve all existing
   retired identity reservations.
5. Reconstruct normally, then verify old canonical declaration IDs/classes/ownership
   and unaffected reference targets, permitting only the declared additions or
   rename. Require exact full-reconstruction equivalence over the same authorized
   source identity inputs and ordinary validation before durable CAS commit.
6. Prove restart restoration retains the same IDs and that a second edit cannot
   resurrect a retired declaration. Record the mapping policy in durable source
   identity history; presentation matching must never substitute for this proof.

## Bounded service implementation

`ModelingService::prepare_part_insertion` now implements this policy only for an
appended, plain named PartUsage, optionally typed by one qualified name. It requires
a Validated predecessor, exact branch head, selected authored owner provenance and
one edit at that owner's final closing brace or semicolon. The agent's existing
CreatePartUsage command uses this preparation path; callers supply no checkpoint,
syntax identity map or retired-ID exceptions.

The service parses the after-bytes and proves a complete one-to-one mapping of old
production kinds and transformed ranges, with the same old child order and owners.
The semicolon-to-body case trims only replacement wrapper trivia for productions
that previously spanned exactly the replaced semicolon. Declaration headers and
all disjoint source remain byte-identical. Exactly one new plain PartUsage is
permitted and every new production must lie inside the insertion. The existing
syntax checkpoint shape validator independently rechecks the derived arena.

The service clones only the authenticated predecessor's remaining checkpoint
fields, preserves all retired reservations, and invokes ordinary full source
reconstruction. It then checks every old authored canonical ID, class, owner and
declared slot value, permitting appended owned relationships only on the selected
owner. Previous reference relationships must remain complete with the same
targets; shadowing is rejected. The new Part must have the exact original owner.
Ordinary validation and durable compare-and-swap remain mandatory for commit.

The durable manifest records policy `agentique-source-identity/part-insertion/1`,
before/after source and syntax-arena hashes, source revisions, owner, added syntax
identity, and the explicit mode `full-source-reconstruction`. Validation retains
this metadata. It is evidence of the command policy, not a new semantic authority.
The normal durable source checkpoint stores the reconciled identities for restart.
Frozen producer/source-input files and accepted receipts are unchanged.

This solves an identity correctness prerequisite, not the requested incremental
semantic speedup. The performance oracle compares full command preparation against
a direct full rebuild over identical reconciled source identities. It continues
to require exact canonical records, references, query evidence and closure identity.
The real platform gate now requires owner and ancestor continuity, new Part owner,
commit and durable restart; real accepted-runtime execution is still pending.
RenameElement and CreateConnection remain unsupported.

## Actual verification boundary

```text
rustfmt --edition 2024 crates/modeling-agent/src/commands.rs
exit 0
git diff --check
exit 0
```

The integration lead ran `cargo test --release --config profile.release.lto=false
--locked --offline -p agq-modeling-agent --lib -j 2`: all eight tests passed,
exit 0. Both identity-churn regressions reproduced. See the exact
[command/output receipt](checks/source-command-identity-tests.json).

The first service-proof gate found a real
semicolon wrapper-trivia mismatch (2 passed, 1 failed). After the bounded fix,
`cargo test --locked --offline -p agq-modeling-service --lib part_insertion::tests -- --nocapture`
passed all 3 tests, exit 0, in 2.094 seconds. The exact receipt is
`checks/part-insertion-proof-tests-boundary.json`. A new sequential-insertion test
reconstructs the first mapped arena before the second insertion and requires the
first new Part's identity to remain; its test run is pending at this commit.

The temporary
Rename implementation was committed only in the isolated worktree and explicitly
reverted before handoff; neither commit should be cherry-picked. Only the retained
regression and subsequent bounded insertion implementation are integration candidates.
