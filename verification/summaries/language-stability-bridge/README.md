# Language stability and modeling platform bridge

Base: fetched `origin/main` at
`0fbdf3ee19788a260b99d89e71dbd28f432d8243`, with a clean starting worktree.
Integration branch: `foundation/language-stability-and-modeling-platform-bridge`.
Normative inputs remain pinned KerML 1.0 / SysML 2.0. Accepted KerML Operational
v9 is unchanged; generation-1 release records are independent.

## Verification in progress

`commands.json` records actual ordinary commands, exit codes, tested source
identities, environment overrides and output hashes. `publication-commands.json`
records watchdog observations. Raw output is ignored under
`verification/generated/`; no successful log is acceptance authority.

The original Actions/dependency baseline was stopped deliberately after 490.266
seconds when it repeated the known discarded-evidence diagnostics and the first
concrete closure fix was ready. Exit 124 denotes a requested workflow stop, not a
semantic result. Peak private memory was 4,515.0 MiB. No final reference audit was
produced, and provisional endpoint selections are not Complete reference counts.
The full-publication attempt budget is unused.

The integrated packages passed 134 KerML semantic library tests (two existing
profiling probes ignored) and 36 text tests, including exact 21-document declared
construction, all 1,327 source assertions, publication rejection gates, immutable
KerML sharing, and four self-model tests. The self-model result is still current-
graph structural acceptance; see [its separate record](../agentique-self-model/README.md).

Initial closure/rebinding tests and their precise boundary are recorded in
[closure.md](closure.md). Independent review reproduced unresolved source-provider,
causal-reader and delayed owning-carrier counterexamples. The corrected producer
mask propagation passes 53 focused closure tests, including those regressions.
Four corrected eight-document Actions audits finished normally, with exit 1 and
no watchdog stop. Each parses/constructs 8/8 documents and selects all 534
endpoints with zero kernel obligations. The latest reaches **499/534 mandatory
references Complete**, with 35 Incomplete and every other failure category zero.
Producer scheduling converges but remains Incomplete. These are failed scoped
gates, not Systems acceptance. Medium/full publication has not been attempted.

| Scoped run | Seconds | Peak private MiB | Final diagnostics |
| --- | ---: | ---: | --- |
| `actions-provider-closure` | 795.172 | 4,984.4 | 3 owner-type closure, 217 producer closure, 13 value-context, 71 variable featuring |
| `actions-declared-source` | 1,013.563 | 5,211.5 | 3 owner-type closure, 219 producer closure, 13 value-context, 66 variable featuring |
| `actions-selected-populations` | 1,066.390 | 5,037.0 | 1 owner-type closure, 169 producer closure, 13 value-context, 62 variable featuring |
| `actions-positive-witness` | 788.969 | 4,955.0 | 1 owner-type closure, 122 producer closure, 13 value-context, 62 variable featuring |

The latest certificate has 8,259 applicable subject/family pairs, 5,107 closed and
208 explicitly incomplete. Its compact proof is 2,567,512 bytes; optional in-memory
revalidation reads are separately measured at 272,315,080 bytes. The latter remain
an optimization target, not part of the compact receipt-size claim. Two checked
rebindings retain 992 evaluations and reopen 8,912; the final predicate pass's
individual zero counts do not summarize earlier reconstruction revalidation.
The report includes per-subject closure explanations for diagnosis.

Independent review subsequently reproduced same-aggregate/different-original-slot
certificate aliasing and a mixed current/source read that could miss an append.
The integrated correction binds opted-in producer contexts and checkpoints to
original slots and preserves mixed current reads. Historical accepted KerML graph
identity remains unchanged. Focused regressions, full KerML/SysML package suites,
Clippy and strict Rustdoc pass; see [closure.md](closure.md). Selected original
ownership witnesses and explicit owned-end populations then close the reproduced
unnamed constraint/connection fixture and improve the scoped count by 24 references.
The remaining formal-owner finding concerns the ForLoopAction while-loop body.
Another corpus run waits for a concrete correction to the remaining causal closure
failure. The latest release binary includes selected positive specialization
witnesses and precise positional population evidence. The small loop regression
closes, including a true mayTimeVary body and shared snapshot featuring; the
remaining difference from the real library is still under diagnosis. The live
publication binding context fix is separately covered by rejection and attachment
tests and does not establish publication acceptance.

Subsequent bounded regressions separate structural valuation from contextual
value binding and assign exclusive derivation-rule ownership to producer families.
The loop-owner and assertion-body fixtures now close. Transition suppression
retains one proven canonical payload chain instead of unrelated ancestor proofs.
Exact kernel append-contribution evidence removes unrelated ownership append
proofs while preserving adoption guards, broad reads and conservative archive
fallback. Four independent semantic regressions and three-package Clippy pass;
the first test drafts required explicit guard slots and compact-proof assertions,
and their failed runs remain recorded. The receiver fixture still has seven
Incomplete producer pairs because its newly created helper's initial ordered
slot requires the same precise support. These fixes have not yet been measured
in another corpus audit.

The existing `AGQ_PRODUCER_CAUSAL_TRACE=1` diagnostic can be bounded with
`AGQ_PRODUCER_CAUSAL_SUBJECTS=<comma-separated element UUIDs>`. It records causal
edges involving those subjects; propagation and certification are unchanged.
Package Clippy with warnings denied passes for this diagnostic-only change.

A focused immutable-dependency review passes five tests. Accepted records are
not producer subjects, but combined project searches over local noncomposite
carriers remain subject to local closure. An accepted dependency source marker
does not certify those searches by itself. The existing regressions cover both
external source relationships and noncomposite inverse navigation; no accepted
KerML interpretation or publication was changed.

The medium candidate's static dependency closure is 13 documents: Actions,
Attributes, Calculations, Connections, Constraints, Flows, Interfaces, Items,
Parts, Ports, Requirements, States and Views. `Interfaces::excludingOnce` is a
CalculationDefinition, so its implicit standard base requires Calculations even
without a source import. Static inventory is not semantic closure evidence.

Frontend `npm run check`, `npm run build`, `npm test`, and `npm run standards:check`
passed. KerML/SysML grammar stale checks, the complete metamodel stale check and
both language metamodel runtime gates also passed. An initial interactive
`cargo fmt --all -- --check` returned 1 for newly
written Rust layout; `cargo fmt --all` returned 0 and applied that formatting.
Final integration checks follow after the implementation converges. Browser-facing
code is unchanged, so browser tests are outside this milestone's requested scope.

The [Gen1/Gen2 audit](../gen1-gen2-audit/README.md), proposed
[ADR 0026](../../../docs/adr/0026-language-foundation-stability-contract.md), and
[workspace design](../modeling-workspace-phase1/README.md) are integrated. ADR
adoption and production workspace integration remain gated on the explicit
language-readiness criteria; unrelated full-conformance coverage is not a gate.

The prepared Systems restoration implementation is integrated with an empty
trusted-authority catalogue. Its exact-byte, context, registry and archive
rejection checks pass; actual accepted-cache restoration remains gated. See
[the preparation record](systems-restore-scaffold.md).

The accepted self-model/case harness compiles, and its case source preflight
passes one real-frontend test. The first command used an unqualified exact test
filter and selected zero tests; the corrected command is separately recorded.
Two workspace edit-input preflights pass, including the generated 100-document
source set. The prospective workspace integration test bodies are present, but
there is no production workspace crate or manifest yet and those bodies have not
been type-checked. These preparations establish neither accepted authored closure
nor the readiness gate.

The publication example now selects the normal preparation API for a full
non-audit run. Scoped and audit-only runs still require their final construction
predicate pass. Strict full publication retains its independent unrestricted
scheduler and every acceptance check; this avoids an extra scoped final pass
before the same strict boundary. Independent routing review found no gate bypass.
On success, the report retains construction observations separately and exposes
the accepted certificate and mandatory-reference totals as the final result.
Example Clippy with warnings denied passes. This tooling preparation has not
consumed a full-publication attempt.

An independent C1 source audit found all 69 Systems bindings serve implemented
algorithmic roles: 66 direct targets and three Operational v2 correction targets.
The manifest covers canonical/path/class/library/visibility/source identities,
both publications, profiles and closure identity. No concrete schema gap was
found. Actual accepted bindings and adversarial restoration still require the
real full publication; the source audit is not a positive acceptance result.
