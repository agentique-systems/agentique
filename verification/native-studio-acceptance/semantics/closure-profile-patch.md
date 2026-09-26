# Isolated closure profiling patch

`closure-profile.patch` is an unapplied diagnostic patch. It was prepared from
the commit and exact file hashes recorded in `closure-profile-patch.json`.
The active checkout's production files were not edited. Copies live under
`verification/generated/native-studio-acceptance/closure-profile-draft/`.

The latest real restore still spends 37.723 s in final closure and 47.949 s in
the effective audit. Existing certificate-build timing includes construction
of the next graph's semantic context, so it cannot identify certificate-only
cost. The immutable family-row shortcut does not remove context authentication,
topology construction, blocker propagation or cache-frontier decoding.

The patch adds two disjoint scheduler measurements: round-start context setup
and the next-context subset inside the existing certificate-build interval.
It preserves the old inclusive certificate metric. It also partitions cached
frontier restoration into metadata authentication, kernel decode/validation and
graph-context authentication. A final closure summary separates snapshot setup,
checkpoint capture, release of construction data, cache restoration, scheduler
rounds and the remaining unattributed interval.

Set `AGENTIQUE_SOURCE_CLOSURE_TRACE=1` on the instrumented helper. Lines prefixed
`SOURCE_CLOSURE_PROFILE` contain JSON with format
`agentique-source-closure-profile/1` and one of three boundaries:

- `cache_frontier`: disjoint internal restoration phases.
- `scheduler_round`: existing planning, dependency-index, materialization and
  revalidation timings, plus the context/certificate split and exact call counts.
- `final_closure`: the outer partition and unchanged scheduler counters.

The cache and round records are nested breakdowns of the outer closure record.
Do not sum child and parent totals. Within a round, sum context setup, planning,
dependency indexing, materialization, revalidation, inclusive certificate build
and unattributed time. Alternatively replace inclusive certificate build with
certificate-context time plus exclusive certificate build. The exclusive field
still includes initial certificate issuance, which does not construct another
context. Source checkpoint rebind work, if present in a scheduler callback, is
inside that callback's context time and must not be counted a second time.

Only successfully completed phases produce records. Failed semantic operations
continue to fail normally. Logging errors are ignored and no timing affects
query results, producer scheduling, closure evidence, acceptance, cache identity
or durable state. Large retained proof structures are not formatted or logged.

After the active build/oracle window, the integration lead can apply with:

```text
git apply --check verification/native-studio-acceptance/semantics/closure-profile.patch
git apply verification/native-studio-acceptance/semantics/closure-profile.patch
```

Build the existing `profile_open` helper with its recorded release/verification
configuration and repeat the same database/runtime invocation from
`../read-profile-02/measurement.json`, with the trace environment variable set.
Use the normal process-memory sampler. Retain executable/source identities and
the full output, including unsuccessful runs. The current patch has not been
compiled or benchmarked: targeted `rustfmt` on all four isolated files and
`git apply --check` both returned exit code 0. Their records and before/after
hashes are in `closure-profile-patch.json`.

The next implementation decision depends on those timings. If initial and first
certificate issuance repeat significant topology work, use the same private
`ClosureCertificateBuilder` cache for the initial certificate while preserving
its empty evaluation table, exact registry checks and full-certificate oracle.
If context authentication dominates, retain exact immutable context inputs
instead. This patch makes neither optimization and claims no latency reduction.
