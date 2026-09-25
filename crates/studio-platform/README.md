# Native Studio platform boundary

This crate is the toolkit-independent in-process application boundary. It reuses
`agq-modeling-service`, `agq-modeling-view`, `agq-modeling-agent` and
`agq-runtime-publications`. Native UI code does not access SQLite tables,
semantic scheduler state, or mutable kernel storage.

Create `NativeConfig::for_root(root, runtime_dir)`, optionally override `database`
and runtime inputs, and call `setup_surface` for pre-authentication setup data.
Run `open` or operator-triggered `install` on a background worker. Progress is
reported through `BootstrapPhase`. Both publications must authenticate before
the durable project opens. `StudioPlatform` and its projection results are Send;
the worker owns the facade and returns DTOs over the UI's channel.

After opening, `projects` and `history` supply the chooser and revision timeline.
Use `RevisionBinding { project, revision }` for `project`, `inspect`, `explain`,
`source`, `dependencies` and `compare`. Selection and camera remain UI state.
Switching a branch or revision establishes an explicit new binding. Consumers
must discard stale worker responses when their selected binding changes.

`propose(AgentContext, ModelCommand, ViewDefinition)` returns a process-local
`CandidateId` plus a Working revision projection, source preview and semantic
diff. Only the reviewed `CreatePartUsage` source mapping is supported by the
existing modeling-agent contract. Unsupported rename/delete/connection commands
are rejected by that contract. `candidate`, `inspect_candidate` and
`explain_candidate` and `source_candidate` address the candidate revision without pretending it is a
durable branch head. `validate` requires independent validation authority;
`commit` requires commit authority and an actually validated candidate.

Commit attempts retain the exact service operation for durable CAS and retry.
An unresolved acknowledgement cannot be cancelled or rewritten. `cancel` drops
an uncommitted candidate; it never creates semantic undo history. Eight active
review handles and 32 total handles bound process-local retention. Durable
operation receipts remain in the repository.
An explicit atomic CAS conflict proves the commit was refused, so the candidate
remains validated and may be cancelled or replaced with a fresh reviewed proposal.

`StudioPlatform::new(service, AgentPolicy::agent(...))` supports an embedded
participant with Read + Propose only. Policies are trusted host inputs, never
provider-generated payloads. Native operator bootstrap installs operator policy.
External Systems Modeling API/HTTP clients retain their separate adapters.

The existing first-run importer was extracted without semantic changes and is
shared by native and web hosts, including its durable bootstrap-intent journal.
Runtime authentication code and standards authority were not copied into UI.

Fixture scenes belong to the native presentation layer and never enter this
facade as canonical semantic truth. Real runtime acceptance remains separate
from fixture interaction tests.

Focused lifecycle tests inject storage acknowledgement loss and CAS refusal at
the retained-operation boundary. They verify deny-before-work authority, failed
validation retention, no presentation-only validation substitute, acknowledgement
retry, idempotent receipt replay, and candidate pressure preserving unresolved
operations. The ignored `native_in_process_self_model_candidate_commit_and_restore`
test supplies the separate real-model gate: revision-bound projection/inspection/
explanation, source for a newly created candidate, immutable parent, validation,
CAS conflict/cancellation, durable commit and process restoration. Run it only
with an existing accepted runtime; it never republishes standards.
