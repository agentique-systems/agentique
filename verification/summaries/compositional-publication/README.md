# Compositional publication

Base: clean fetched `origin/main`, `068f7b0995825e84ffb04cb9937437ba15fd5ebb`.
Branch: `foundation/compositional-publication-and-modeling-workspace`.
[ADR 0027](../../../docs/adr/0027-compositional-semantic-publication.md) was
committed before implementation. Independent planner, certificate and equivalence
work used isolated `agq-comp-plan`, `agq-comp-cert` and `agq-comp-audit` worktrees.
Long-path checkout initially failed; enabling repository-local Git long-path
support allowed clean isolated checkouts. No existing worktree was reset.

## Acceptance boundary

Compositional closure is **incomplete**. Dependency planning and read-only
component audits do not seal strata, issue a composite publication certificate,
authenticate resumable checkpoints or establish the medium equivalence gate.
No new full Systems producer run is permitted by these diagnostic results.

The existing certificate conservatively opens every non-typing requirement
affected by an unfinished producer across the local graph. Its source comment
states that precise footprints for those query families remain unimplemented.
Current producer write scopes alone cannot replace that read-side proof.
Inverse/global negative searches and future subjects are additional explicit
dependencies. An immutable earlier Element record is not proof that a later
local relationship cannot change an incoming search.

The implementation therefore retains these guards in its SCC plan and requires
the existing scheduler certificate's actual local pairs, requirements and read
evidence in its component audit. Neither a caller-provided partition nor matching
counts can promote it to acceptance. The strengthened equivalence fixture checks
logical canonical content and evidence, not traversal counters.

Nine planner tests cover SCC ordering, linear/diamond/cyclic dependencies,
canonical typing/subsetting carriers, fresh writers, unknown and negative reads,
input permutation identity and authenticated external providers. In particular,
an immutable mount or an advertised digest without closure authority cannot hide
an open provider. Six actual scheduler component-audit tests distinguish a truly
unsafe model-wide writer from the conservative guard on a disjoint subject-scoped
writer: the latter leaves every earlier local pair closed but its membership
requirement open. A zero-output negative search can change captured read evidence
without changing the flat receipt, so a new component proof must bind those reads.

The exact comparator's five mutation controls reject equal-count changes to
proofs, searches, failures and requirements. Existing full-scan/worklist fixtures
continue to pass with this stronger comparison. These tests do not execute a
compositional scheduler. The [independent boundary review](audit-boundary.md)
also records the ordered-contribution evidence that checkpoint restore must
preserve; a graph archive alone is insufficient for the proposed new proof.

## Actual Systems dependency probe

The release planning fixture passed in **201.5 seconds**, peak private memory
**4,607.0 MiB**, with no watchdog stop. It restored the accepted KerML publication,
parsed all 21 original Operational v2 documents byte-exactly, constructed their
declared graph and refined source references. It invoked no producer scheduler.
The earlier unoptimized probe was deliberately stopped during cache restoration
after 283.0 seconds (exit 124, 3,817.6 MiB peak); it yielded no planning result.

The provisional SCC plan contains **one component**:

```text
component 0
  documents: Actions, Allocations, AnalysisCases, Attributes, Calculations,
    Cases, Connections, Constraints, Flows, Interfaces, Items, Metadata,
    Parts, Ports, Requirements, StandardViewDefinitions, States, SysML,
    UseCases, VerificationCases, Views
  local predecessor components: none
  known immutable external provider: accepted KerML Operational v9
  declared subjects: 7,591
  applicable declared producer pairs: 20,274
  mandatory source references: 1,327
  explicit dependency edges: 601,470
  provider edges: 577,207
  edges to accepted dependency subjects: 404,181
  potential writer rows: 70,608 (8,830 from future subjects)
  writer rows with current global requirement guards: 70,608
  unbounded record-writer rows: 0
  cross-component reads / writable effects: 0 / 0
```

This is a conservative proof-dependency plan, **not** a proved minimal semantic
partition of the final closed Systems graph. Generic query reads retain broad
invalidation dependencies; no source-reference read set supplies the missing
producer-family evidence. The plan records 1,327 global provider observations,
2,871 subjects missing producer evidence and 12 open provider requirements.
Its zero cross-component counts follow from placing everything in one component;
they do not prove an independently sealable boundary.

This fresh declared-only graph has 1,323 selected endpoints, four kernel
construction obligations, and reference outcomes of 1,252 Complete / 75
Incomplete. Producers were intentionally not run, so these are not a regression
or replacement for the earlier completed construction audit below. No component
sealed, and no certificate composition/equivalence claim follows from this probe.

The generated report is `verification/generated/compositional-publication/systems-plan.json`,
SHA-256 `34d849cf78d2f499d868d75ff53bfbf28e23dd952a8422c3da024ce7bde11f5c`.
It retains source-byte, accepted-dependency, context, registry, graph and plan
identities. Commands and resource limits are in the consolidated ledger.

## Publication and readiness

`SYSML SYSTEMS LIBRARY CANONICAL PUBLICATION INCOMPLETE`

The last completed full **construction** audit on the base recorded 1,327/1,327
reference assertions passing and zero kernel obligations. That remains historical
construction evidence, not a newly issued strict publication certificate. The
last strict attempt did not finish. Accepted Systems bindings and trusted receipt
remain unavailable; accepted KerML Operational v9 is unchanged.

`AGENTIQUE LANGUAGE FOUNDATION NOT YET STABLE`

ADR 0026 remains proposed. Agentique self-model and effective SysML acceptance
against an accepted Systems publication remain unrun. Full SysML conformance is
not an extra readiness gate.

`MODELING WORKSPACE PHASE 1 REMAINS GATED`

No production modeling workspace crate or platform integrations are introduced.
Gen1 release obligations and original authority/library bytes are unchanged.

## Reproduction and remaining proof

The final integrated checks all exited **0**:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | 900 passed, 0 failed, 5 ignored across 126 test groups |
| `npm run check` | Pass |
| `npm run build` | Pass |
| `npm test` | Pass |
| `npm run test:e2e` | Pass, 4 browser tests |
| `npm run standards:check` | Pass |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | Pass |
| `cargo doc --locked --offline --no-deps -p agq-kerml-semantics -p agq-kerml-text` | Pass with `RUSTDOCFLAGS=-Dwarnings` |

The workspace test command ran once on integrated source commit `14941b5`,
with one build job and one test thread in the low-artifact environment; it took
1,307.12 seconds. Explicitly ignored corpus/cache tests were not silently counted
as passes. The separate bounded Systems planning fixture above was run explicitly.
Initial focused development failures, a corrected zero-test filter attempt and
the deliberately stopped unoptimized probe remain in the ledger. Subsequent
focused checks passed. These integration results verify the implemented changes;
they do not establish compositional publication acceptance.

Actual verification commands, source identities, output hashes and exit codes
are recorded in `commands.json`; raw output stays under ignored
`verification/generated/compositional-publication/` and the established watchdog
storage. Focused agent ledgers are consolidated into that file with their source
metadata. Every copied raw output was checked against its recorded SHA-256;
PowerShell's UTF-16 certificate-workstream captures retain their original bytes.

The bounded corpus planning test requires `AGQ_ACCEPTED_KERML_CACHE` pointing to
the pinned accepted cache and `AGQ_COMPONENT_PLAN_REPORT` pointing to ignored
generated output. It parses and constructs original sources, refines declared
references and plans dependencies without invoking any producer scheduler. Its
construction/reference observations do not replace the earlier completed
construction audit or the final publication audit.

Before a component can seal, the remaining work is to establish complete bounded
requirement/provider footprints (including zero-output and future-writer cases),
bind them to private component issuance, and integrate immutable stratum storage
and authenticated checkpoint restore. Then execute exact three-strategy synthetic
equivalence and medium monolithic/compositional equivalence. Only a passing medium
result permits component Systems closure and the final global read-only audit.
