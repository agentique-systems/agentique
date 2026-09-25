# Completed platform projections: held qualification

This patch adds a 16-entry LRU inside one StudioPlatform instance. Its key is the
full exact `RevisionBinding` and `ViewDefinition`, including definition version,
name, kind, graph scope, focus, depth, standard visibility and ordered family/
hidden-element lists. Entries retain only `Arc<ViewProjection>`; they do not
retain a BoundRevision, canonical model, query evaluator, candidate or authority.

Every call first executes the existing `self.bound(binding)` path. Read policy
and ordinary service resolution therefore precede lookup even for a hit. The
resolved manifest's project/revision must match the request. Returned outer,
node and edge revisions and the entire returned view definition are checked
before retention and again on a hit. An unavailable binding cannot use an old
cached DTO. Different projects, revisions or platform instances cannot collide.

Queries, full DTO identity checks, cloning, concurrent-completion comparison and
evicted DTO destruction run outside the mutex. The lock only covers bounded key
lookup and LRU/Arc bookkeeping. Concurrent equal misses may compute twice; one
result is retained. A disagreeing completion fails instead of overwriting the
first result. Errors are never cached. A poisoned cache bypasses retention and
still requires normal resolution and successful checked computation.

Warnings, producer completeness, groups, features and relationships are retained
verbatim. A completed DTO that reports incomplete semantics remains incomplete.
Caller changes affect only an owned clone. Compare and candidate paths still
invoke modeling-view directly; this cache is not an alternate candidate binding
or an optimization of paired diff construction. The existing dependencies method
uses project with its exact dependency view definition, so it benefits without
losing its traversal scope.

`AGENTIQUE_VIEW_PROFILE=1` emits separate
`agentique-studio-projection-cache/1` / `platform_project` events. They distinguish
`hit`, `miss` and `not_consulted` (resolution/identity refused before lookup).
Wall time includes policy/binding, lookup, result checks and DTO cloning; misses
also include modeling-view work. It excludes worker queue time and profiling
serialization/emission. Do not add this enclosing duration to nested query
phases or present a cache hit as zero-time semantic query execution.

This does not remove waiting behind a mutation or another worker request.
Ordinary binding may itself restore an evicted service revision, even when the
projection DTO is retained. Retention is bounded by entry count, not byte size;
sixteen very large views can consume substantial memory. No latency or peak
memory improvement is claimed until measured with real projections.

## Verification prepared

Nine cache-mechanics tests cover exact full DTO/serialization and caller-copy
isolation; resolution on every hit; every definition field and ordered filters;
project/revision/platform isolation; read refusal and failed/foreign binding;
wrong returned outer/nested identities; no failure caching; LRU eviction;
equal/conflicting concurrent completion; and poison bypass. Inert closures and
explicit unit-only DTOs cannot mint an authenticated BoundRevision or constitute
semantic acceptance.

The existing ignored platform real-model gate additionally compares its actual
first/repeated project results, serialized bytes and caller-copy isolation. It
adds no runtime restoration, new fixture or model mutation. The temporary caller
copy is dropped immediately; the normal bounded cache remains as it would in the
product. Prior real Inspector/restart/CAS/rename obligations stay intact.
The existing comparison's uncached `before` result still establishes fresh old
revision projection equality, and the source-restored predecessor's semantic
fingerprint is compared around the second edit. Cached presentation equality
therefore cannot replace the prior canonical immutability obligations.

Only rustfmt and source/diff checks were performed for this held patch. No build,
unit test, accepted runtime, view timing or semantic gate has run for it yet.
After baseline native acceptance and an allocated build/runtime slot:

```powershell
cargo test --locked --offline -p agq-studio-platform --lib projection_cache::tests -- --test-threads=1
cargo clippy --locked --offline -p agq-studio-platform --all-targets -- -D warnings
```

Use the existing release-build/measurement helpers to run
`native_in_process_self_model_candidate_commit_and_restore` once with this patch
when the real platform durability gate is scheduled. Keep the real native
journey and query oracle as independent gates; repeated loaded-view measurements
must identify the same exact revision and full definition.

No language source, accepted-input pin, model source, dependency or lockfile changes.
