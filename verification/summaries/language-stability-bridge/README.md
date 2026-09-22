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
The corrected eight-document Actions audit finished normally in 795.172 seconds
(exit 1, no watchdog stop; peak private memory 4,984.4 MiB). All eight documents
parse and construct. Final selection is 534/534 with zero kernel obligations, but
only **469/534 mandatory references are Complete**; 65 are Incomplete and every
other failure category is zero. Producer scheduling converges but remains
Incomplete: 3 formal owner-type closure, 217 producer closure, 13 value-context
and 71 variable-featuring diagnostics. This is a failed scoped gate, not Systems
acceptance. Medium/full publication has not been attempted.

The final certificate has 8,224 applicable subject/family pairs, 4,122 closed and
365 explicitly incomplete. Its compact proof is 2,565,702 bytes; optional in-memory
revalidation reads are separately measured at 307,744,640 bytes. The latter remain
an optimization target, not part of the compact receipt-size claim. Final-pass
transport counts are zero because that pass starts a separate predicate regime;
they do not summarize earlier reconstruction revalidation.

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
