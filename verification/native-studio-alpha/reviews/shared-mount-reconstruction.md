# Shared accepted dependency during proven source reconstruction

Status: implementation prepared in an isolated worktree; **not compiled, not run,
not independently reviewed, and not approved for integration**. The lead required
baseline measurement before changing the active runtime consumer. No performance
improvement or runtime equivalence is claimed by this record.

## Exact implementation scope

| File | Change |
| --- | --- |
| `crates/kerml-text/src/source_checkpoint.rs` | Add `SourceIdentityCheckpoint::restore_sharing_dependency` and an optional predecessor in the existing private restoration helper. Cold `restore` and `restore_cached` pass no predecessor. A verification-only method observes mount Arc identity. |
| `crates/modeling-workspace/src/checkpoint.rs` | Add a successor-only wrapper with exact parent and distinct candidate revision checks; return a Working handle. |
| `crates/modeling-workspace/src/testing.rs` | Expose the verification-only mount-sharing observation through the workspace boundary. |
| `crates/modeling-service/src/source_identity.rs` | Change the internally proven source command's single restoration call to the new successor wrapper. |
| `crates/modeling-agent/tests/create_part_performance.rs` | Require an independent cold checkpoint reconstruction, exact checkpoint and semantic equality, and 16 malformed-request refusals. Report the hot-command and cold-restore measurement boundaries explicitly. |

The service continues constructing its own reconciliation checkpoint from the
actual predecessor. CreatePartUsage and bounded RenameElement retain their
existing identity proofs, declared-slot allowances, owner/reference postchecks,
ordinary validation and durable CAS. The stored policy/evidence mode remains
`full-source-reconstruction`, which is still true.

## Why the proposed sharing boundary is sound

`SourceCompilation` contains a private `Arc<AcceptedSourceDependency>` minted by
the accepted language facade. Its mounted standard graph and closure witness are
immutable. The new method derives the publication facade directly from that
predecessor; callers cannot supply another facade or a completeness flag.

Before constructing inputs, the existing format, accepted KerML/System digest
and source population checks still run. The new path additionally requires the
checkpoint's exact source project and canonical root to match the predecessor.
The outer wrapper requires its exact project revision as the sole parent and
refuses reuse of that revision as the candidate identity.

Fresh `SourceInputs` use the same default parser limits as cold restoration,
empty document storage, and the cloned authenticated dependency. Every source
digest, document/path population, syntax identity/shape, identity reservation and
source ledger passes through the unchanged common restoration logic. The final
call is still `compile_with_history(None, restored_history, false, None)`:

- No predecessor local graph, lowering cache or producer frontier is supplied.
- No local semantic cache, prior audit result or local closure certificate is reused.
- Declaration construction, reference refinement, strict kernel validation,
  producers, closure/certification, final references and effective audit all run.
- Working remains distinct from Validated; no durable acknowledgement occurs.

Pointer equality is observed only under the verification feature. It is neither
a semantic identity nor an acceptance shortcut.

## Required proof before acceptance

The existing `full_rebuild` helper reuses a compilation's inputs and mount, so it
cannot independently qualify this change. The updated real CreatePartUsage gate
instead calls the **unchanged cold** `checkpoint.restore(publication, sources)`
using the candidate's exact checkpoint and source bytes. It requires:

- Hot candidate and predecessor share the exact authenticated mount; cold oracle
  has a different mount. This checks actual reuse without granting authority.
- Exact source/arena/retired-identity checkpoint equality.
- Exact canonical records, ElementIds, provenance and relationship ordering.
- Equal semantic fingerprint, context contract and closure semantic identity.
- Equal reference results, completeness, diagnostics, positive/negative search
  evidence and explanations, plus the retained effective-query oracle.
- Both reconstructions validate; neither used a source semantic cache; both
  reparsed the entire source-document population.
- Wrong checkpoint/source version, parent, candidate revision, source project,
  canonical root, accepted publication digest, source population/content/identity,
  document path/identity or syntax arena must fail. Sixteen cases are retained.

The durable source-only restart tests continue using cold restoration. Old
revision immutability and exact owner/ancestor continuity remain existing gates.

## Measurement boundary

`CompilationTimings` is unchanged. Report v3 records hot command wall time, cold
checkpoint-restore wall time, both complete compile profiles and both validation
times. The command additionally includes source proof and service postchecks;
cold restore independently authenticates its mount. Their noncompile residuals
are explicitly mixed overhead, **not mount measurements**. This patch eliminates
one repeated standard-dependency mount; it does not eliminate local semantic
compilation work or establish incremental semantic speedup.

The historical 167,012 ms versus 95,220 ms observation remains historical. Peak
memory is null unless externally measured, and must state whole-process scope.

## Source freshness and authority

ADR 0026 permits implementation/API changes preserving the identity, revision,
query and evidence contracts. Repository source freshness remains a separate
review gate. Among interpretation inputs, only `source_checkpoint.rs` changes:

```text
Recorded normalized SHA-256:
eb86d74ead07e68664081b131c3dbd152877fdb12c1374d3ba2175764a57aa0e
Proposed normalized SHA-256:
1b77839611609533c329b800843aca3e97530b1c8e011f8c42c3ddd593261e07
```

`standards/sysml-publication-inputs.json` is deliberately **not updated**. After
independent code review and exact cold equivalence, the lead may review that one
fingerprint replacement while asserting all publication identity, receipt,
binding, Cargo.lock and other input entries remain byte/semantically unchanged
as applicable. No blanket freshness recapture is appropriate. Producer/rule/
profile identities, normative/library inputs, accepted receipts and transport
pins are untouched. Until that reviewed update, the freshness gate should fail.

## Checks and next commands

Actual local checks: `cargo fmt --all -- --check` exited 0, recorded in
`checks/shared-mount-source-format.*`; `git diff --check` exited 0. No build,
accepted cache restoration or semantic consumer ran in this worktree.

After the lead preserves the baseline and approves integration, use the existing
target and one compiler job. Relevant commands include:

```text
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-agent --features verification --test create_part_performance --no-run -j 1
cargo clippy --locked --offline -p agq-modeling-service -p agq-modeling-workspace -p agq-modeling-agent --features agq-modeling-agent/verification --all-targets -j 1 -- -D warnings
cargo doc --locked --offline --no-deps -p agq-kerml-text -p agq-modeling-workspace -j 1
```

Only when no other standards consumer is running, supply the installed exact
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE` paths and run:

```text
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-agent --features verification --test create_part_performance create_part_command_matches_full_self_model_reconstruction -- --exact --ignored --nocapture --test-threads=1
```

Retain actual command/output/exit and externally sampled memory. A compile or a
matching work count does not close the performance or semantic-equivalence gate.
