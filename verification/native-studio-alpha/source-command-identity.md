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

## Minimal correct contract for a later implementation

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

This is a proposed contract, not an implemented or accepted identity policy. The
frozen producer/source-input files remain unchanged during runtime recovery.
Adding CreateConnection before resolving its owner's identity would inherit the
same risk, so connection breadth was not substituted for this defect.

## Actual verification boundary

```text
rustfmt --edition 2024 crates/modeling-agent/src/commands.rs
exit 0
git diff --check
exit 0
```

The integration lead runs the focused modeling-agent Cargo test against the shared
target. No test pass is claimed at this document's initial commit. The temporary
Rename implementation was committed only in the isolated worktree and explicitly
reverted before handoff; neither commit should be cherry-picked. Only the retained
regression and this review are integration candidates.
