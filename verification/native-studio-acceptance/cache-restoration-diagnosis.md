# Cache restoration diagnosis and bounded next step

## Measured follow-up, 2026-09-26

The retained real profiles now confirm the source-inspection diagnosis below.
See `read-profile-02/README.md` and its raw command output. The same immutable
repository revision opened in 193.691 s before the producer/audit optimizations
and 181.258 s afterward. This is not the required dramatic warm-open improvement.

The latest cache blob authentication took 32 ms and compressed decoding/frontier
hashing 208 ms. In contrast, source compilation still took 108.494 s: source
preparation 14.702 s, strict kernel validation 620 ms, final closure 37.723 s,
final reference resolution 7.439 s and effective audit 47.949 s. It rebuilt
1,101 declared records, evaluated 332 producer subjects and audited all 791
local subjects. Persisted restoration has no in-memory predecessor audit to
reuse. Expensive source-derived semantic work, not compressed byte decoding,
dominates this path.

Runtime publication restoration adds 65.722 s. The trace includes large accepted
archive decode/kernel-validation passes and recanonicalization against the pinned
publication identity. Those are distinct from the project cache. Loaded project
reads are already much cheaper: System focus 377 ms, Graph and Requirements
about 60 ms, cached projections about 1 ms. The native journey separately
observed 132 ms durable repository commit and retained-candidate validation
without redoing the whole audit. These improvements do not establish faster
project restoration or candidate construction.

The historical draft descriptions below record the reasoning before integration;
they are not a current list of untested changes. The source-only restoration gate
has since passed again in a separate process after deleting the disposable cache.
The next isolated measurements partition context/certificate costs and test audit
batch lifetime with full query-value/evidence/completeness equivalence. Neither
experiment has produced a speed result yet.

## Initial source inspection

This is a source inspection, not a new wall-clock result. The integration lead is running the first instrumented real project opening; its measurements determine which draft to integrate. No v3 acceptance shortcut is implemented here.

## Why an authenticated cache still resembles reconstruction

The compressed `agq-project-semantic-cache/2` format (`modeling-service/src/cache_codec.rs`) transports the existing logical v1 `SourceSemanticCache`: local effective frontier plus source/model/context/registry/closure/dependency digests. It retains neither the producer certificate/transport reads nor the completed effective audit. Compression alone does not change the restoration boundary.

`SourceIdentityCheckpoint::restore_cached` (`kerml-text/src/source_checkpoint.rs`) checks the checkpoint/frontier digests, then enters `restore_maybe_cached`. That path mounts an accepted dependency, reparses every authored document, restores syntax/declared reservations and calls `compile_with_history(..., false, cache)`. Parsing and dependency mounting occur before `CompilationTimings.total_compile_micros` starts.

`SourceInputs::compile_with_history` always calls `prepare_accepted_source`, which rebuilds declarations and performs reference refinement and preparatory producer closure. The semantic cache is first consumed by `finish_accepted_source` in `kerml-text/src/sysml/source.rs`. The cached branch decodes against the source-derived strict declarations and invokes the final producer scheduler again. Afterward `SourceInputs` runs the strict authored effective audit on all local subjects in batches. Ordinary workspace `validate()` checks this retained audit and certificate; it does not itself rerun the entire effective audit.

The kernel bound-frontier archive already carries declarations, reservations, effective records and proof/search material. `read_bound_frontier_on` checks them against the caller's source-derived snapshot. Its decoder temporarily constructs declared indexes before comparing and replacing that snapshot; after decoding it constructs effective indexes. Immutable standard dependency indexes are shared, so describing this as blindly rebuilding all 76k global indexes would be inaccurate.

## Safe draft implemented

The cached final-closure branch previously computed a source-derived producer checkpoint and then ignored it. It now uses the same `checkpoint.rebind(exact_restored_context, independently_constructed_registry)` path as uncached reconstruction. Rebind examines actual positive, negative/provider and context reads; affected populations reopen. The scheduler, exact cache graph/context/closure digest checks, source references and complete effective audit remain in place. Cache bytes do not supply an accepted producer state.

The existing accepted-runtime checkpoint test now retains reference query values/completeness/evidence and audit subject/count populations across restoration, and prints retained/reopened closure work and compilation phases. It also checks that the full audit still visited its entire original population. Its existing stale-source/corrupt-byte rejection and source fallback checks remain. These ignored runtime checks require the real accepted publication caches and are not claimed as executed in this worktree.

## Measurement map

- `AGENTIQUE_STARTUP_PROFILE=1` records manifest authentication, source/identity loading, cache blob hashing, compressed decode/frontier digest, source-bound graph/closure/audit restore, semantic identity authentication, and final durable validation receipt checks. Nested totals overlap; do not sum both inner and outer totals.
- `agentique-restored-compilation-profile/1` records identity preparation, declared construction, reference refinement, preparatory producers, strict kernel validation, final closure, final references, and effective audit, plus actual work counters.
- The difference between the outer restoration phase and compile total includes dependency mounting, source authentication and parsing. Add timers around those separately if that residual dominates.
- Accepted runtime `RuntimeTimings` separates source verification, KerML restore and SysML restore. Existing runtime restore authenticates archive entries, decodes/validates, then re-encodes the actual constructed graph to match the separately compiled accepted receipt. These repeated traversals deserve measurement, but the final graph check cannot simply be removed.
- `profile_open` measures actual authenticated process opening and revision-bound projections/Inspector first and repeat. It intentionally excludes native layout, scene building, GPU upload and displayed frame; those must come from native journey evidence.

## V3 direction and authority boundary

A future v3 should persist the local bound kernel frontier, source maps/reference evidence, the complete closure certificate and transport reads, and a completed strict effective-audit record with an explicit audit implementation contract and exact local subject population. The existing `ConvergedPublicationFrontier::authenticate` and internal `FrontierCertificate` transport show how to restore graph/context/registry-bound certificate state without producer execution. These APIs confer no publication or project validation authority.

The outer gate must require an independently authenticated durable Validated revision receipt, not a receipt supplied by the cache itself: exact project revision, source binding/checkpoint/source bytes, accepted publications, implementation contract, model/context/closure identities and the cache blob digest pinned by that immutable manifest. Working revisions and any mismatch use ordinary source reconstruction. Rebind fresh runtime revision labels only after comparing semantic context identities; never use stored timing/count labels as proof of acceptance. No reconstructed audit may inherit incremental reuse evidence that was not persisted and authenticated.

Today `RevisionManifest` and `ValidationReceipt` are public deserializable data. Their content hashes establish consistency but are not an opaque validation capability. Exposing a new public language function that accepts arbitrary matching hashes and manufactures a completed audit would let callers bypass `WorkingProjectRevision::validate()`. The integration lead therefore deferred that trust-boundary change. A precise repository-owned restoration capability/authority contract is required before a v3 path can skip the effective audit. The current draft does not expose such a bypass.

If source preparation dominates after the bounded seed fix, v3 should also restore declared state from the already-present bound frontier under that same authority, retaining all normal kernel archive/provenance/protected-dependency checks. Reparse the small authored source set for editing identities initially; do not add AST persistence before measuring it. Persistence of additional indexes is a later, separately measured optimization.

An independent safe read-path opportunity is retaining `CanonicalSysmlSystemsLibrary::producer_closed_dependency()` on its immutable accepted facade. It currently reconstructs a composed context and rechecks complete closure for each new source history. A facade-owned OnceLock would share only exact immutable authenticated state, with no receipt trust expansion; measure dependency mounting first.

## Verification

Local draft commands: `rustfmt --edition 2024 crates/kerml-text/src/sysml/source.rs crates/modeling-workspace/tests/phase2_checkpoint.rs`, exit 0, empty output; `git diff --check`, exit 0, line-ending notices only. Compilation and real cache restoration checks await the integration build window. The native interaction retest independently reported 145 passing tests and one remaining Worlds layout failure; all camera, navigation and candidate/read cancellation checks passed.
