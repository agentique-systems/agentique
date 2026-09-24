# Independent finalizer and profile review

Reviewed integration commit `7903065` with the shared population audit from
`cc0f993`, as present under `9a48199`. This was a read-only code review; it does
not establish publication acceptance or replace the required runtime gates.

No acceptance bypass or silent profile downgrade was found:

- `close_for_finalization` retains the existing full-population, input,
  obligation, source-provenance, strict-storage and binding checks. Its result
  has private fields and exposes only borrowed scheduler evidence. Both ordinary
  publication and direct finalization still call the unchanged `accept_closed`.
- `--close-only` requires a full population and checkpoint session, creates no
  candidate artifact transaction, and reports publication as unattempted and
  unaccepted even when strict producer closure succeeds.
- Direct finalization authenticates the selected profile in the source identity
  before restoring the journal, reconstructs exact source declarations under
  that profile, rebuilds the registry independently, and authenticates the exact
  graph/context certificate. It does not replay producers.
- `accept_closed` still requires converged Complete closure and complete
  certificate coverage, KerML capability/provenance audits, authority audit,
  all mandatory references, verified bindings, and the effective SysML audit
  before constructing the accepted facade.
- Cache restore authenticates an explicit compiled receipt for the selected
  profile, checks its exact manifest/rule-set/dependency identities, validates
  every document's profile, authenticates archive bytes and the decoded graph,
  and attaches only the matching certificate. Candidate-cache verification
  remains available only from an already accepted live facade. The compiled
  Systems catalogue remains empty.
- Scoped candidate auditing calls the same `audit_sysml_population` function as
  finalization. That function includes current and effective Usage, Occurrence,
  Item, Part and Connection definition queries. The scoped constructor rebuilds
  the expected registry and dependency contract before attaching the draft's
  exact scheduler certificate; open requirements remain explicit findings.

One tooling inconsistency was corrected after review: v2 can have historical
`agq-sysml-query/5` or current `agq-sysml-query/6` evidence because implementation
identity is distinct from operational interpretation. V3 permits only `/6`.
The Python artifact gate now admits exactly those pairs, with matching receipt,
binding, manifest, graph and independent certificate checks retained. Tests
exercise all three admitted pairs and reject unknown versions and v3 with `/5`.

## Bounded performance inspection

No semantic or audit implementation was changed for performance. The current
audit keeps at most two workers and eight subjects per query context, joins in
input order, and drops full query evidence before merging compact reports.
Context forks reuse frozen identities rather than fingerprinting the graph.
Those memory and determinism properties should be retained.

Static candidates to compare with the actual combined-slice/medium timings:

- Each two-batch window creates and joins new OS threads. A persistent pair of
  workers could eliminate that overhead while still creating fresh bounded
  query contexts per batch and preserving ordered report merging. Benefit is
  unmeasured and likely small when semantic evaluation dominates.
- For a ConnectionUsage, current/effective projections repeatedly reconstruct
  the same Usage-to-Occurrence-to-Item prefix. KerML queries reuse their cache,
  but SysML adapters repeat type pruning, proof-vector assembly and typed fact
  observations. A bounded shared projection prefix inside the query layer is a
  possible follow-up only if profiling identifies it; every public audit API,
  diagnostic, closure check and evidence payload must remain equivalent.
- `close_population` revisits ancestor/usage closure implications across several
  effective operations. Underlying KerML results are cached, while SysML proof
  assembly is repeated. Any cache here must be bound to the immutable context
  and preserve operation-specific negative evidence. Do not skip the checks or
  retain corpus-sized memo tables merely to improve timing.

No performance conclusion is inferred from static inspection. The next decision
should use the updated medium audit's measured duration and existing per-batch
timings; semantic correctness remains the gate.

Inspection commands included `git show --stat --oneline 7903065`,
`git diff 7903065^ 7903065 -- crates/kerml-text/src/sysml/publication.rs crates/sysml-semantics/src/context.rs`,
targeted PowerShell `Get-Content ... | Select-Object` reads and `rg -n` searches
over finalization, restoration, query and context modules (exit 0). Two initial
searches used nonexistent/glob paths and returned exit 1; corrected explicit
paths and directory searches were used before drawing these conclusions.
