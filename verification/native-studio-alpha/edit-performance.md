# Candidate reconstruction diagnosis

No end-to-end semantic latency improvement is claimed by this workstream. The
accepted runtime is being rematerialized independently; the new real-model gate
has not yet run. No semantic producer, kernel index, publication receipt,
authority decision or frozen publication input was changed.

## What the existing measurements establish

The prior accepted-cache oracle at commit
`068fc4412452f5faf9f8fee6fdba0dfb1328d951` measured add-part at **167,012 ms
incremental versus 95,220 ms full authored reconstruction**. Both paths evaluated
141 final-closure producer subjects and audited 116 effective subjects. The
incremental path lowered one document (41 records); the full path traversed six
document passes (98 records). The full seven-edit process peaked at 5,078,302,720
bytes of Windows peak working set. That is whole-process memory, including
accepted-publication restoration and all edits/oracles, not per-edit allocation.

See [the original oracle](../summaries/modeling-platform-phase2/frontend-acceptance.json).
These are historical observations, not rerun alpha results. They demonstrate
that reduced lowering work did not establish reduced wall time.

The later 401-to-9 result used a **100-document rename construction fixture**,
not CreatePartUsage with accepted semantic closure. Its 24.721 ms versus
161.748 ms comparison excluded producer closure, effective audit and persistence.
See [its explicit measurement boundary](../summaries/agentique-studio-phase3/performance.md).

There is no retained current accepted-cache phase profile from which to name the
dominant phase honestly. Current `CompilationTimings` already separates reference
refinement, declared construction, preparatory producers, strict kernel validation,
final closure/certification, final references and effective audit. Nested timings
overlap source preparation and must not be added indiscriminately.

## Why speculative reuse was rejected

- Lowering already reuses unchanged documents; declared reconstruction already
  submits changed/new/removed records against the strict predecessor.
- Kernel validation and local index reconstruction still process the combined
  candidate. Skipping these needs invariant-specific dependency boundaries.
- Producer checkpoint rebinding authenticates actual reads, including negative
  searches. The historical equal final evaluation counts make its overhead worth
  measuring, but do not prove that rebinding is the dominant current phase.
- Effective audit stores results and subjects, not complete reusable successful
  query footprints or future-writer exclusion. An unchanged subject record cannot
  authorize reusing its effective answer after another element changes.
- `source_inputs.rs`, `sysml/source.rs`, construction/refinement and the semantic
  engine are recorded in `standards/sysml-publication-inputs.json`. Changing their
  authority/freshness input bytes during accepted-cache recovery would require
  separate contract handling; this workstream does not alter those files.

## Focused real command gate

`agq-modeling-agent/tests/create_part_performance.rs` exercises the actual
`ModelCommand::CreatePartUsage` source mapping, ModelingService, durable repository
and current `models/agentique/*.sysml`. It requires a Validated baseline, constructs
a Working candidate under ModelingPlatform, confirms the durable head and old
semantic fingerprint remain unchanged, and compares against `full_rebuild` over
the exact same parsed source identities. Both results must validate.

The comparison checks canonical records, IDs, provenance, relationship order,
semantic fingerprint, closure semantic digest, reference values/completeness and
positive/negative/proof evidence, diagnostics, and the created part's effective
query evidence. It does not substitute an aggregate count for equivalence.

After runtime authentication, supply the exact installed cache paths through
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`, then record:

```text
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-agent --features verification --test create_part_performance create_part_command_matches_full_self_model_reconstruction -- --exact --ignored --nocapture --test-threads=1
```

The `CREATE_PART_PERFORMANCE` JSON line separates total command preparation,
incremental/full compile phases and validation. Work retained/eliminated remains
alongside wall time. Runtime restoration is separate. Peak memory is explicitly
null unless an external process monitor measures it. Compare compile totals with
compile totals: full reconstruction excludes parsing and command/service overhead.
The gate makes no commit, restart or complete operator-journey claim.

Focused compile/test execution is delegated to the integration lead to avoid
competing builds against the shared target. Local `rustfmt --edition 2024
crates/modeling-agent/tests/create_part_performance.rs` completed successfully;
integrated check results must be recorded before this gate is called verified.

## RenameElement feasibility at the initial audit

The parser's `production::Document::edit` deliberately assigns fresh identities
to nodes overlapping an edit; `authored_id` derives the canonical ElementId from
SyntaxNodeId and an output role. Replacing a declaration's name therefore retires
its identity and may also replace overlapping ancestor declarations. An arbitrary
text replacement is not a stable-identity RenameElement command.

A sound command needs explicit declaration identity reconciliation bound to the
old exact source revision, collision and scope checks, and exact full-rebuild
comparison under the authorized identity mapping. That initial audit did not
enable rename. The later service-owned `part_rename.rs` proof now supports a
bounded plain-name Part rename with zero additions/removals and exact previous
reference targets. It refuses renames requiring a reference rewrite. Its
source-only restart and real-runtime checks remain separate gates; it is not an
incremental performance result.

## Current identity-preserving command path

The service's `source_identity::reconstruct` clones the prior source checkpoint,
installs its internally proven syntax arena for the edited document, and calls
`ProjectRevisionCheckpoint::restore`. That reaches
`SourceIdentityCheckpoint::restore_maybe_cached`, which:

1. Checks the accepted KerML/SysML identities and source population.
2. Calls `SourceInputs::with_accepted_sysml`, mounting a newly authenticated
   `ProducerClosedDependency` over the same immutable publication overlay.
3. Checks every source digest, reparses every document, and restores each checked
   syntax identity arena. No inherited elements are copied by this mount.
4. Restores identity reservations and the source ledger.
5. Calls `compile_with_history(None, ..., false, None)`: no predecessor lowering
   cache, no incremental producer reuse and no source semantic cache.
6. Runs service continuity postchecks and serializes the Working candidate.

The command oracle correctly labels this a full source reconstruction. Its
`full_rebuild` comparison reuses the exact parsed input and already mounted
dependency, disables lowering/producer reuse, and repeats the semantic compile.
Thus `command_prepare_ms / full_rebuild_ms` is **not** an incremental speedup
ratio. Compare the two compile totals separately from command overhead.

The compile timers start after the dependency mount, source authentication and
parsing. `command_prepare_ms - command_compile_ms` includes those operations,
source proof, continuity checks, repository reads and candidate serialization.
It cannot be reported as dependency-mount time. Source preparation contains the
declared construction, reference refinement and preparatory producer timers;
do not add those nested values again to the total.

## Reuse boundary and measurable candidates

There is no currently exposed API for restoring service-proven identities while
sharing the predecessor's mounted dependency. `SourceInputs`' dependency,
project/root and document fields and `compile_with_history` are private.
`ProjectChange` has no authenticated-arena variant. Ordinary `workspace.prepare`
uses `Document::edit`, which replaces overlapping ancestor syntax identities;
substituting it would violate the insertion proof's owner continuity contract.
`source_checkpoint.rs`, `source_inputs.rs`, `sysml/source.rs` and the publication
facade are frozen publication inputs. No changes to those files are proposed
during accepted runtime installation.

One bounded opportunity exists outside those files: service
`prepare_candidate(validate=true)` performs `WorkingProjectRevision::validate`
and obtains a genuine `ValidatedProjectRevision`, then callers such as
`PreparedChanges::validate`, `prepare_changes`, and `prepare_part_insertion`
validate the exact same immutable `Arc` again. A private service helper returning
both the candidate and that checked handle could remove the duplicate call
without skipping any check. The handle must remain bound to the exact Arc;
callers must never supply a validation boolean as substitute evidence. Source
reconstruction, effective audit, cache authentication, candidate verification,
and durable CAS would all remain mandatory. This is only a candidate for
measurement: its wall-time significance is unknown, and it cannot by itself
establish incremental reconstruction speedup.

Once the runtime installation gates finish, run the retained command oracle
serially before changing code. Retain command/compile/full/validation time and
phase data, exact-equivalence assertions and externally sampled process memory.
If command overhead dominates, add service-boundary observations around source
proof, checkpoint restoration, continuity checks and candidate packaging. If
compilation dominates, identify the largest non-overlapping phase; do not infer
that audit or certificate work is reusable merely because source changes are
small. This review ran no builds and consumed no accepted runtime assets.
