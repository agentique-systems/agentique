# Language stability and modeling platform bridge

Base: fetched `origin/main` at `0fbdf3ee19788a260b99d89e71dbd28f432d8243`
with a clean starting worktree. Branch:
`foundation/language-stability-and-modeling-platform-bridge`.
Inputs remain pinned KerML 1.0 / SysML 2.0; accepted KerML Operational v9 `/26`
is unchanged. Generation-1 release evidence remains independent.

## Current gate status

**Systems publication and foundation readiness remain unaccepted.** The latest
completed corpus result is the eight-document Actions/dependency scope:
**534/534 mandatory references Complete**, zero unresolved, incomplete,
ambiguous, invalid or mismatched references, and zero kernel obligations. All eight
documents parse and construct byte-exactly. The scheduler converges but producer
closure remains Incomplete: four evaluations remain incomplete and the certificate
is not fully closed. The scoped acceptance guard correctly rejects this result.
Medium/full publication has not run;
the full-publication attempt budget is unused.

All six genuine layered expression fixtures and the connection-end fixture pass
with fully closed certificates. The indexed assignment now matches the real
frontend and authentic Action/Occurrence ancestry. Actual-planner and independent
guards verify the end-membership filter, directed valuation proof separation,
eligible argument populations and the earlier per-effect/immediate owning-Type
writer scopes. These synthetic dependency results are not publication acceptance.
The rebuilt `actions-end-and-index` audit reduces the remaining incomplete
evaluations from 24 to four; it remains an unsuccessful acceptance gate.
See [closure.md](closure.md) for each correction and exact evidence boundary.

The [Gen1/Gen2 audit](../gen1-gen2-audit/README.md), proposed
[ADR 0026](../../../docs/adr/0026-language-foundation-stability-contract.md) and
[workspace design](../modeling-workspace-phase1/README.md) are integrated. ADR
adoption and production workspace integration remain gated by the explicit
readiness criteria. Unrelated full-conformance coverage is not an extra gate.
The finite Systems receipt catalogue is empty; actual accepted bindings and cache
restoration remain gated. [restore.md](restore.md) records C1/C2 preparation and
the executable actual-cache gate.

## Publication observations

The latest `actions-end-and-index` audit uses clean source `0e3d7f5` and the
successful release build from `905e4ee`. It exits 1 normally in **1,409.547 seconds**,
peaking at **6,124.8 MiB private memory**, with no watchdog stop. All eight documents
parse/construct byte-exactly; selected endpoints and Complete mandatory references
are both 534, with zero reference failures, kernel obligations or authority
conflicts. Accepted KerML producers are not replayed.

Closure converges in 31 rounds but remains Incomplete and not fully closed:
**four MayTimeVary pairs**, four diagnostics, 11,044/11,547 closed applicable pairs
and 382,623/407,490 closed requirements. It evaluates 20,030 subjects and derives
3,418 elements. Certificate/revalidation sizes are 2,631,923 and 682,310,664 bytes;
two rebindings retain 1,235 and reopen 11,919 evaluations in aggregate.

The indexed assignment and sixteen previously blocked ends now close. The four
remaining subjects are Flows lines 68/69 (`[1] source` and `[1] target`) and Items
lines 138/139 (the two `[0..*]` item ends). Original KPAR hashes were checked before
mapping their source spans. Exact command, exit, resource observations, final
counts and report hashes are in [publication-commands.json](publication-commands.json).
Earlier failed audits remain historical in that ledger and [closure.md](closure.md).
Medium/full publication has not run; the full-attempt budget is unused.

Full non-audit publication now uses normal preparation; scoped/audit-only paths
retain the final construction predicate pass. Strict publication independently
reruns the unrestricted scheduler and every acceptance check. On success, reports
keep construction observations separate from accepted closure/reference totals.
Routing review found no bypass and example Clippy passed; this tooling change has
not consumed a full-publication attempt.

## Medium candidate preflight

The static scope is **13 documents**:
`Actions,Attributes,Calculations,Connections,Constraints,Flows,Interfaces,Items,Parts,Ports,Requirements,States,Views`.
`Interfaces::excludingOnce` is a CalculationDefinition; its implicit base requires
`Calculations::Calculation` even without a source import. This corrects the earlier
12-document proposal. Calculations adds Actions and accepted KerML Performances.

Read-only inspection of pinned Systems source bytes removed comments and collected
qualified prefixes matching Systems document names, then compared actual
metaclasses with `BASE_RULES` and binding-role paths. The explicit dependencies are:

| Document | Explicit Systems dependencies |
| --- | --- |
| Actions | Flows |
| Attributes | none |
| Calculations | Actions |
| Connections | Actions, Parts |
| Constraints | none |
| Flows | Actions |
| Interfaces | Connections, Ports |
| Items | Constraints, Parts |
| Parts | Actions, Items, Ports, States |
| Ports | none |
| Requirements | Actions, Attributes, Constraints, Interfaces, Parts |
| States | Actions |
| Views | Parts, Requirements |

Producer targets, contextual owned/composite rules, subaction/transition/state and
Operational v2 Viewpoint rules introduce no additional Systems package. KerML
roles use the separate immutable publication. This is a static preflight, not
reference completeness or closure evidence; run medium only after Actions passes.

## Integration verification and retained evidence

The integrated KerML semantic package at clean source `3506712` passes: **394
tests passed, zero failures**, two existing release-only scale probes ignored.
All 36 suites reported success; the one-thread, one-build-job, low-artifact command
exited 0 in 295.03 seconds. Its exact command and source/output hashes are retained
in `commands.json`. Later descriptor changes require their own focused gates.

At clean source `749f9c9`, the SysML semantic and KerML text package command
passes **176 tests** (81 SysML, 95 text), zero failures, with only the two explicit
accepted-cache/self-model gates ignored. All 17 suites succeed with native Cargo
exit 0. Exact output/hash, the exit observation and per-suite runtimes are retained;
runner wall duration was unavailable. Frontend check, build, Node tests and
standards integrity also pass at source `c83d7f0`.

Early integration passed 134 KerML semantic library tests (two existing profiling
ignores) and 36 text tests, including exact 21-document construction and all 1,327
source assertions, rejection gates, shared KerML and four self-model tests. Those
are baseline counts, not the final package population. The self-model checks so
far establish current-graph structure; see [its record](../agentique-self-model/README.md).
The accepted self-model/case harness compiles and its real-frontend case preflight
passes. An initial unqualified exact filter selected zero tests before the
corrected command. Two workspace edit-input preflights pass, including 100 mixed
documents; the prospective workspace test bodies have not been type-checked
because the production crate/manifest remains gated.

Frontend `npm run check`, `npm run build`, `npm test`, and `npm run standards:check`
passed. Both grammar stale gates, the complete metamodel stale gate and both
language runtime gates passed. An initial `cargo fmt --all -- --check` exited 1;
`cargo fmt --all` exited 0 and corrected layout. Integrated workspace/all-target Clippy with warnings denied, strict Rustdoc
for seven language crates, and the dependency-boundary gate now pass. The final
workspace test run remains a separate integration obligation. No browser-facing code changed,
so browser tests are outside this milestone's requested scope.

[commands.json](commands.json) records ordinary commands, exits, exact tested
source/patch identities, environment overrides and output hashes.
[publication-commands.json](publication-commands.json) retains watchdog observations.
The ordinary ledger preserves all original and subsequent review records.
`source_ledgers` preserves each original
filename, source Git commit, Git-blob SHA-256 and zero-based command range.
Consolidation checked exact record equality for every range and retained duplicate
names/runs; the publication ledger was left unchanged. [closure.md](closure.md)
and [restore.md](restore.md) retain the findings and Markdown-only observations.
Markdown-only observations explicitly mark missing hashes/identities rather than
inventing them. Raw logs and reports are ignored under `verification/generated/`.
A finished command, zero-test filter or compile-only fixture is not acceptance.

Bounded diagnostics use `AGQ_PRODUCER_CAUSAL_TRACE=1` and optional
`AGQ_PRODUCER_CAUSAL_SUBJECTS=<comma-separated element UUIDs>`; they only restrict
logging. Five immutable-boundary checks confirm accepted records are not local
producer subjects while qualifying local source/inverse carriers remain open.
No accepted KerML identity or interpretation changed.
