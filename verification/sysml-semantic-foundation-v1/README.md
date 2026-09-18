# SysML semantic foundation v1

Result: **SYSML SEMANTIC FOUNDATION INCOMPLETE — DO NOT START PLATFORM**.

Base: `190588dd1ed84cd49a437aec83a6d933ff9e968a`, fetched `origin/main` on
2026-09-18. The initial worktree was clean. Branch:
`semantics/sysml-foundation-v1`. No platform or execution work is authorized here.

Evidence directories are append-only. `run.mjs` records complete process output,
actual exit codes, command lines and duration. Successful structural runtime
checks do not establish language semantic completion. Strict metamodel authoring
conformance is a separate check and its published anomalies remain errors.

`authority.py` builds/checks the complete 93-class semantic inventory directly
from pinned XMI and the supplied specification. Retained bodies are evidence,
not an OCL evaluator. Library dependency names in this inventory are normative
binding requirements, never parser builtins or fabricated library declarations.

## Implemented boundary

- Complete 93-class/384-rule-operation XMI authority inventory and ADR 0012.
- Multi-document KerML source project over the existing generation-2 lowering
  stack: atomic publication, immutable revisions, one root/snapshot, stable
  unrelated IDs, explicit replacement/move policy and source evidence.
- Mixed source retention with an explicitly unavailable SysML frontend; affected
  namespace resolution stays incomplete. Recovery regions also preserve this
  boundary. Resolution and KerML semantic diagnostic domains are distinct.
- Exact offline four-KPAR source loader: all archive/entry hashes, project/meta
  records, dependency versions and cycles, retained archive discrepancies/junk,
  immutable verified inputs and private deterministic provenance-based identities.
- Lossless lexical audit of all 57 textual entries and normative decimal/
  exponential tokens. This is explicitly not a complete grammar-production inventory.
- Negative namespace-search and 49-document inheritance regressions. No inherited
  feature is copied; the original ownership and membership remain intact.

No canonical library graph, complete library parser, import foundation,
`agq-sysml-semantics`, SysML frontend, Definition/Usage rules or structural SysML
semantic family is implemented. The requested semantic milestone remains
incomplete because implementation remains, not because approval is required.
Every requested gate is tracked in `standards/v2-coverage.json`; the full review
is `docs/sysml-semantic-foundation-review.md`.

## Recorded results

`verified-results.json` summarizes the actual recorded commands; it does not
override any process exit code. `final/results.json` records all 16 normal commands
at exit zero. This includes the full requested workspace/application matrix,
both complete structural runtime checks, strict Rustdoc for all seven generation-2
public crates plus the generator, authority/corpus currentness and independent
structural verification. Cargo reports 214 passed tests including doctests,
Node reports 10, and Playwright reports 4. No tests failed or were ignored.

| Evidence | Outcome |
| --- | --- |
| `gate-0/` | Authority inventory and both runtime gates: zero |
| `gate-1-project/` | Initial project tests, formatting and runtime gates: zero |
| `gate-2-source-loader/` | Loader, numeric syntax, corpus, Clippy and runtime checks: zero |
| `project-regressions/`, `recovery-regressions/` | Identity, dependency, stress and recovery regressions: zero |
| `final/` | 16 normal commands: zero |
| `strict-final/` | Both strict metamodel conformance commands: **1** |
| `quality-final/` | Library semantic quality command: **1** |

The 4 KerML and 12 SysML strict conformance errors are identical, including
dispositions, to `language-core-completion-v3/strict-current`. They remain errors;
several SysML dispositions are still unreviewed. Structural runtime readiness
does not convert strict conformance into success.

`library-quality.json` reports all 57 documents. All have zero lexical-probe
errors, but all 36 KerML sources require parser recovery and all 21 SysML sources
lack a SysML frontend. Lowering/resolution/semantic validation were not performed.
Null counts mean not evaluated; zero element counts mean no canonical library
model was built. `--require-semantic` remains a failing gate.

The preservation command checked 648 protected files against the fetched base.
190 supplied/pinned/historical artifact files were compared byte-for-byte;
ordinary Git text was compared with line-ending normalization and its actual
worktree SHA-256 recorded. PDFs, original HTML, library archives and normative
entries were never rewritten. Generation-1 release registers and existing
structural kernel/descriptors/generator remain unchanged.

## Reproduction

Run from the repository root with installed Rust/Node dependencies and Python
`pypdf` for the supplied-PDF authority scan. No command downloads standards.
Use a fresh evidence label because directories are append-only:

```text
node verification/sysml-semantic-foundation-v1/run.mjs final repeat-final
node verification/sysml-semantic-foundation-v1/run.mjs strict repeat-strict
node verification/sysml-semantic-foundation-v1/run.mjs quality repeat-quality
```

The runner sets `CARGO_NET_OFFLINE=true` and `RUSTDOCFLAGS=-D warnings`, records
complete stdout/stderr and actual exits, and continues recording after failure.
Generated application verification artifacts are copied into this milestone's
evidence directory, then their previous bytes are restored. Strict/quality runs
return 1 at this implementation state. Historical failed quality evidence remains
in `gate-2-semantic-quality/`; it has not been replaced by successful checks.
