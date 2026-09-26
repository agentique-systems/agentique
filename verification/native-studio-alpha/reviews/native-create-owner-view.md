# Nested Create must expose the edited owner

Independent diagnosis and bounded native correction, 2026-09-25. Base source:
`98407f8`. This is a source/state review, not real candidate acceptance.

## Concrete defect

The real operator journey selects `ModelingPlatform` in the System overview and
opens Create through the ordinary command palette. It does not focus that owner
first. Previously `open_part_edit` retained only the command target and
`prepare_part` passed the unchanged current `ViewDefinition` to the platform.

The current System overview has depth 1. The ordinary modeling-view projection
uses two forward Ownership/Typing hops per architecture level
(`projection.rs`, `architecture_neighborhood`):

```text
Agentique -> modeling -> ModelingPlatform -> newly owned PartUsage
             hop 1       hop 2              hop 3
```

`StudioPlatform::candidate` projects the actual prepared revision through that
same supplied definition. A valid source edit can therefore return an overview
that omits the new child. The real `CandidateWorking` assertion correctly demands
the actual named PartUsage with its canonical owner; weakening that check or
inserting a frontend node would hide the product defect.

Run04 later stopped before mutation on a separate dependency-view definition
mismatch. Consequently this predicted candidate failure was not observed in that
process. Its existing captures and failure evidence remain untouched.

## Product correction

Opening Create explicitly enters the pinned owner's ordinary System view at
depth 1. The previous camera and navigation location remain available to Back.
The transition clears temporary neighborhood/agent display state, leaves durable
comparison mode, and expands the edited owner. It neither changes global view
depth nor adds semantic records.

The dialog retains the original owner and exact requested definition. Prepare and
Enter share the same readiness check as direct submission: the base binding and
revision, complete view definition, current World/focus, installed scene revision
and owner, and layout context must match; no owner projection or scene build may
remain pending. Existing command capability/Validated branch-head checks still
apply. The dialog explains loading or an incompatible later view intent.

Selecting another visible object does not change the pinned edit owner. A later
World, focus, filter, or neighborhood choice is respected: it disables the old
dialog rather than forcing navigation back or preparing in another scope. A
failed or superseded owner query cannot enable preparation. Normal candidate
construction still produces its real after projection; the current matched owner
projection supplies the before side through the existing guarded response path.

The existing fixture request path now retains its requested presentation
definition before rebuilding. This allows the same readiness contract in visual
fixtures, without a Create-specific bypass. Fixture contents and authority remain
explicitly illustrative. The owner-retention fixture test now selects an actual
visible child in the intentionally focused owner view.

## Qualification

Five new native state regressions cover owner focus/Back, pending or mismatched
scene readiness, later focus/World intent, stale live-query delivery, and rejected
owner scene installation. The existing pinned-owner test now also checks the
matching before/after owner lens. These tests exercise fixture DTO/native routing
only and do not mint accepted-runtime or write authority.

The real `CreateDialog` assertion additionally requires the ordinary readiness
fence before its existing input sequence proceeds. The action sequence and
`CandidateWorking` checks are unchanged.

Retained actual checks:

- `checks/native-create-owner-view-format.json`: focused rustfmt check, exit 0.
- `checks/native-create-owner-view-diff.json`: whitespace check, exit 0.

No native compilation, tests, standards consumer, or real model process was run
in this isolated worktree. Integration tests and a real owner-view → prepare →
visible canonical child → candidate/diff journey remain required.
