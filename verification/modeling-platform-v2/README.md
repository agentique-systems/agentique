# Modeling platform v2: prerequisite blocked

The requested platform is **not implemented**. No platform stage gate has passed.
The task assumes the complete generation-2 SysML language milestone is merged;
the fetched repository contradicts that prerequisite. This record implements the
requested safe stop when continuing would require unsupported language behavior
or weakening semantic invariants. It is not a replacement architecture proposal.

## Checkout and sequence

On 2026-09-18, `git status --short` returned no changes (exit 0).
`git fetch origin main` succeeded (exit 0), and
`git rev-list --left-right --count main...origin/main` returned `0 0` (exit 0).
Both refs identified `3a1375a6628da5d0597442249e7c5f3312db0c2a`, whose subject is
`standards: audit SysML runtime closure and preserve normative blocker`.
`git switch -c platform/modeling-platform-v2 origin/main` succeeded (exit 0).
No commit was made directly to `main`.

The branch contains a prerequisite-evidence commit only. Stages 0 (platform ADR
and database-independent codec) through 6 (integrated documentation/verification)
are not started. No repository, SQLite adapter, API mapping, view or transformation
crate is created. No durable encoding, external identifier policy, API conformance,
vertical-slice acceptance or platform performance result is claimed.

## Evidence and dependency boundary

The [workspace inventory](prerequisite/inventory.txt) lists no `agq-sysml` or
`agq-sysml-semantics`. The [coverage register](../../standards/v2-coverage.json)
records language runtime gate 3 as blocked, and the
[language milestone record](../sysml-language-v2/README.md) marks its later gates
unimplemented. Current text support is the single-document KerML slice;
multi-document source projects, SysML lowering and generation-2 library ingestion
are not available.

The [runtime readiness log](prerequisite/runtime-readiness.txt) reproduces
`SysML runtime closure blocked` with cyclic property metadata and exit **1**.
The [structural audit](../../standards/generated/sysml-2.0/structural-audit.json)
and [existing blocker analysis](../../docs/sysml-v2-runtime-blocker.md) identify
two required self-subsetting properties in the pinned KerML 1.0 / SysML 2.0 inputs:

- `Kernel-Associations-A_targetType_targetAssociation-targetAssociation`
- `Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage`

The existing analysis traces both into even the minimum PartDefinition/PartUsage
closure and records corroborating supplied-PDF evidence. This run confirms the
local failure; it does not claim a new search for external errata.
The [two refusal regression tests](prerequisite/blocker-regressions.txt) pass
(exit 0); that proves refusal behavior, not SysML runtime readiness.

The generation-2 architecture, semantic-kernel and text guides, coverage register
and ADRs 0001-0007 were reviewed before this decision. The required platform consumes
the completed language stack: it cannot supply missing SysML semantics, reinterpret
the normative metadata, or substitute generation-1 models. Starting a reduced
KerML-only repository would change the requested milestone's prerequisite and
would not establish its SysML source/library context or integrated acceptance.
Consequently no platform ADR or persistence contract is marked accepted against
the absent language contracts.

## Reproduction and verification

Run from the repository root:

```sh
node verification/modeling-platform-v2/run.mjs prerequisite
node verification/modeling-platform-v2/run.mjs baseline
```

The prerequisite runner deliberately propagates the failing check, returning 1.
[prerequisite/results.json](prerequisite/results.json) records every actual
command, exit code, duration and stdout/stderr log. Inventory and runtime readiness
both returned 1; the two blocker regression tests returned 0.

The baseline runner records existing workspace regression checks separately from
platform gate acceptance. It uses `CARGO_NET_OFFLINE=true` and
`RUSTDOCFLAGS=-D warnings`, captures logs and reports under this directory, and
restores historical standards/browser report bytes after copying new reports.
All 11 baseline commands returned **0**, including all four browser tests.
[baseline/results.json](baseline/results.json) records exact commands, exit codes,
durations and logs. This is existing-workspace regression evidence, not platform
acceptance. The ordinary generator check confirms current imported artifacts;
its success does not override the failing runtime-readiness check.

| Command | Exit |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | 0 |
| `cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-metamodel-gen` | 0 |
| `node verification/sysml-language-v2/check-docs.mjs` | 0 |
| `npm run standards:check` | 0 |
| `npm run check` | 0 |
| `npm run build` | 0 |
| `npm test` | 0 |
| `npm run test:e2e` | 0 |

There are no new public crates requiring additional Rustdoc targets, and no
repository implementation to exercise restart or synthetic performance tests.

## Resumption and Pack 3

Supply the merged commit containing the completed language milestone, or complete
that milestone first on `main`. Resolution must preserve the pinned evidence and
kernel invariants: the existing blocker calls for an authoritative correction
applicable to KerML 1.0 / SysML 2.0, or a separately authorized and explicitly
non-conformant compatibility policy. This task does not authorize inventing either.
Then complete runtime descriptors/views, semantics, source projects, SysML text,
library ingestion and language acceptance before restarting platform stage 0.

Re-fetch and verify the prerequisite on the actual completed base; review this
evidence-only branch rather than treating it as the platform implementation.
Execute and commit platform stages 0-6 sequentially. Keep the repository codec,
canonical model, API DTO, presentation graph and transformation result distinct.
Retain the user's commit-pinned identity, durability and acceptance obligations.

Defer Pack 3 execution/simulation until both the language prerequisite and all
platform gates are complete. Generation 1 remains the operational product; no
language or runtime migration was attempted.
