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
closure remains Incomplete: 64 evaluations remain incomplete and the certificate
is not fully closed. The scoped acceptance guard correctly rejects this result.
Medium/full publication has not run;
the full-publication attempt budget is unused.

All four genuine layered expression fixtures now pass with fully closed
certificates: plain and frontend-shaped invocation results, a compound chain,
and a deeper nested chain. The actual planner and independent adversarial tests
confirm precise per-effect and immediate owning-Type writer scopes. These are
synthetic dependencies, not accepted standard publications; the corrected
Actions corpus audit remains the next gate. See [closure.md](closure.md) for each defect,
permanent regression and its exact evidence boundary.

The [Gen1/Gen2 audit](../gen1-gen2-audit/README.md), proposed
[ADR 0026](../../../docs/adr/0026-language-foundation-stability-contract.md) and
[workspace design](../modeling-workspace-phase1/README.md) are integrated. ADR
adoption and production workspace integration remain gated by the explicit
readiness criteria. Unrelated full-conformance coverage is not an extra gate.
The finite Systems receipt catalogue is empty; actual accepted bindings and cache
restoration remain gated. [restore.md](restore.md) records C1/C2 preparation and
the executable actual-cache gate.

## Publication observations

All completed scoped runs are failed acceptance gates until the full certificate
closes. The immediately preceding sealed-proof audit had 534/534 references
Complete and 103 incomplete producer evaluations; the current run below has 64.
Earlier failed and deliberately stopped observations, actual exits and resource
limits remain in [publication-commands.json](publication-commands.json) and the
[closure record](closure.md). A requested watchdog stop has no final semantic
result; provisional endpoint selections are never Complete-reference counts.

The latest `actions-bounded-writers` audit (source `695e67e`, release build
`b56a1b5`) finishes normally with exit 1 in 1,180.313 seconds, peaking at
5,821.1 MiB private memory. All 534 references remain Complete, with zero kernel
obligations or authority conflicts. Producer closure converges in 26 rounds but
remains Incomplete: 64 explicit incomplete pairs (42 Usage mayTimeVary,
19 FeatureReferenceExpression, 3 FeatureValue), 67 diagnostics and
380,711/405,270 closed requirements. It evaluates 12,011 subjects and derives
3,048 elements. The 2,617,585-byte compact certificate retains optional
590,341,256-byte revalidation data; reconstruction retains 1,225 evaluations and
reopens 10,975 across two rebindings. No medium/full attempt is consumed.

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
