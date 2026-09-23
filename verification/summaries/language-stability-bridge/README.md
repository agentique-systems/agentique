# Language stability and modeling platform bridge

Base: fetched `origin/main` at `0fbdf3ee19788a260b99d89e71dbd28f432d8243`
with a clean starting worktree. Branch:
`foundation/language-stability-and-modeling-platform-bridge`.
Inputs remain pinned KerML 1.0 / SysML 2.0; accepted KerML Operational v9 `/26`
is unchanged. Generation-1 release evidence remains independent.

## Current gate status

**Systems publication and foundation readiness remain unaccepted.** The latest
completed corpus result is the eight-document Actions/dependency scope:
**532/534 mandatory references Complete, two Incomplete**, zero unresolved,
ambiguous, invalid or mismatched references, zero kernel obligations. All eight
documents parse and construct byte-exactly; all 534 endpoints are selected. The scheduler
converges but closure remains Incomplete. The remaining references are
`Anything` and `receiver` on TriggerAction `35c0b23f-b73e-5c7e-9079-b8c8bc9c39e6`.
Medium/full publication has not run;
the full-publication attempt budget is unused.

Subsequent focused corrections close the while-loop, inherited assertion-result
and trigger/receiver fixtures. The receiver passed in 33.80 seconds on `55b5af8`;
the complete SysML package subsequently passed 73 tests in 71.38 seconds. These
results cover faithful synthetic layered fixtures, not accepted corpus closure.
The completed `08e4a83` audit improves the prior 499/534 result by 33 references
but still fails the scoped gate. No full-publication acceptance is inferred.
A subsequent regression reproduced an unrelated ancestor's variable-property
premise entering an otherwise sufficient Action/SelfLink exclusion proof.
The selected-witness correction (`f15cfca`) passes five focused tests, the receiver
fixture and all 74 SysML package tests; its corrected Actions audit remains pending. The text frontend
package passed 91 tests with two explicit accepted-cache ignores before that fix.
See [closure.md](closure.md) for the defects, proof boundaries, regressions and
historical failed/passing observations. Full KerML verification also passes
336 tests (173 unit, 163 integration), with two existing scale ignores; the
recorded command took 145.84 seconds.

The [Gen1/Gen2 audit](../gen1-gen2-audit/README.md), proposed
[ADR 0026](../../../docs/adr/0026-language-foundation-stability-contract.md) and
[workspace design](../modeling-workspace-phase1/README.md) are integrated. ADR
adoption and production workspace integration remain gated by the explicit
readiness criteria. Unrelated full-conformance coverage is not an extra gate.
The finite Systems receipt catalogue is empty; actual accepted bindings and cache
restoration remain gated. [restore.md](restore.md) records C1/C2 preparation and
the executable actual-cache gate.

## Publication observations

The first Actions baseline was deliberately stopped after 490.266 seconds when
it repeated known discarded-evidence diagnostics and a concrete fix was ready.
Exit 124 is a requested workflow stop, not a semantic result; peak private memory
was 4,515.0 MiB. No final reference audit exists for that run. Provisional endpoint
selections are never Complete-reference counts.

Five corrected scoped audits finished normally with exit 1 and no watchdog stop.
Each parsed/constructed 8/8 documents, selected 534 endpoints and had zero kernel
obligations. These remain failed scoped gates, not Systems acceptance:

| Scoped run | Seconds | Peak private MiB | Final diagnostics |
| --- | ---: | ---: | --- |
| `actions-provider-closure` | 795.172 | 4,984.4 | 3 owner-type, 217 producer-closure, 13 value-context, 71 variable-featuring |
| `actions-declared-source` | 1,013.563 | 5,211.5 | 3 owner-type, 219 producer-closure, 13 value-context, 66 variable-featuring |
| `actions-selected-populations` | 1,066.390 | 5,037.0 | 1 owner-type, 169 producer-closure, 13 value-context, 62 variable-featuring |
| `actions-positive-witness` | 788.969 | 4,955.0 | 1 owner-type, 122 producer-closure, 13 value-context, 62 variable-featuring |
| `actions-exact-contributions` (`08e4a83`) | 894.063 | 5,284.5 | 106 producer-closure, 11 value-context, 45 variable-featuring |

The latest audit has 162 diagnostics, 22 rounds, 10,917 subjects and 2,441 derived
elements. Its certificate records 10,017 applicable subject/family pairs, 6,061
closed and 164 explicitly incomplete, with 379,104 closed requirements. The compact
certificate is 2,594,064 bytes; optional revalidation reads are separately
406,184,536 bytes. Two rebindings retain 1,233 evaluations and reopen 10,419.
A final pass's individual counters do not replace aggregate reconstruction
observations. Per-subject explanations remain diagnostic evidence, not acceptance.

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
The ordinary ledger contains all 112 original records: 47 from the main ledger
and 65 from 13 former review ledgers. `source_ledgers` preserves each original
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
