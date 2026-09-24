# Agentique Studio Phase 3 — incomplete runtime acceptance

Started with a clean `main` worktree. `git fetch origin main` exited 0 and resolved
`origin/main` to `4ab612e1232cfbff6d879e4b72cba0d2447ee47d`. Created
`platform/agentique-studio-visual-agentic-phase3` from that fetched revision.

The first Studio implementation is present, but the requested real-model operator
acceptance sequence cannot be claimed complete. The accepted KerML and Systems
cache files and retained Phase 2 self-model database were absent from this
environment. The three unfinished Phase 2 runtime gates remain blocked. No
publication rebuild, fabricated accepted graph, or sample JSON backend substitutes
for these inputs. See [Phase 2 preflight](phase2-gates.md).

## What the operator can use after accepted inputs are supplied

The new `/studio` application provides a dark/light spatial canvas, semantic
outliner and inspector, System/Graph/Requirements/History/Agents/Source lenses,
three presentation detail levels, pan/zoom/focus, bounded standard expansion,
relationship-family filters, view-only hiding and saved view definitions.
Canonical relationship inspection and Why retain original producer provenance.
Parent-linked branch history switches the immutable revision; visual comparisons
use repository semantic changes and retain removed-object source revisions.

The local host authenticates accepted caches and opens the durable Gen2 SQLite
repository. An empty repository is seeded from the actual Agentique SysML sources,
with an architecture baseline and the ordinary future Agent Fabric concepts.
Working source-backed nested-part candidates are separate from branch heads.
Review includes projected changes and exact source, followed by explicit validation
and operator CAS commit. Restart discards uncommitted review handles; durable
commits and saved view definitions survive.

The deterministic agent routes intent to a semantic view or revision diff. The
provider-neutral Choice/Score/Boolean decision vocabulary has no live provider or
behavioral execution. Programmatic participants can read exact-revision views,
inspect, explain, retrieve source, diff and propose typed source commands with
Read + Propose authority. Validate and Commit are separate operator capabilities.

These are implemented product contracts. The 16-step real durable Studio sequence,
accepted-cache view integration, and accepted-cache candidate integration remain
unexecuted in this environment. This is not full Systems Modeling API conformance
or Generation 1 release completion.

## Actual verification

Actual commands, exit codes, tool versions, source/patch identities and output
hashes are in `commands.json` and the bounded workstream ledgers in this directory.
Raw output and screenshots remain in ignored `verification/generated/`.

| Required check | Actual final result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed; subsequent affected-package checks also passed |
| `cargo test --workspace` | Passed: 1,019 tests, 0 failed, 10 ignored; 832.25 seconds including compilation |
| `npm run check` / `npm run build` | Passed on final frontend |
| `npm test` | Passed: 20 tests after reviewed input-freshness capture |
| `npm run test:e2e` | Passed: 8 tests, including 4 Gen1 and 4 Studio transport contracts |
| `npm run standards:check` | Passed; retained standard bytes and identities unchanged |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | Passed |
| Rustdoc for view, agent and Studio crates, warnings denied | Passed |
| Generation/dependency boundary and boundary regressions | Passed |

The workspace pass was not repeated after the narrow review fixes; the affected
packages were tested independently afterward (64 language-text tests, 6 canonical
view tests, agent/service checks and the Studio host gate). The final four Studio
browser contracts also passed after minor presentation guards were added. Each
command retains its own tested-tree digest rather than implying all commands ran
against one undocumented tree.

- [Frontend evidence](frontend.md): TypeScript/build and four browser transport
  contracts covering selection, revision rejection, filters, evidence, agent view,
  candidate validation/commit/reject and canonical diff changes. Fixtures are
  explicitly labeled and do not grant real semantic acceptance.
- [View evidence](view-engine.md), including the final
  `view-engine-property-resolution-commands.json`: six unit tests over intentional
  projections and strict canonical SysML records; effective property aliases,
  ownership, typing, counts, identities, names and explanation boundaries checked.
  One accepted-cache integration test is compiled and ignored.
- [Agent review](agent-review.md): exact validated-candidate retry payload retained;
  selected typing target checked against its canonical ID; unsupported mappings
  fail. Six agent tests and seven service tests passed.
- [Performance](performance.md): 100-document declared-construction mutation work
  **401 → 9**, with **396 records retained**. All four full-reconstruction oracles
  and 64 language-text library tests pass. Producer and effective-audit frontiers
  remain broad. Whole semantic edit latency is unmeasured.
- [Publication input review](publication-input-review.md): the stale-input gate
  failed before reviewed freshness recapture. Only three implementation hashes
  changed; accepted publication receipts, bindings and semantic identities did not.
- The real `agq-studio` process serves the built application with HTTP 200 and
  reports HTTP 503 for an explicitly missing accepted cache. The host smoke test
  establishes correct startup/error behavior, not semantic viewport acceptance.

The first integrated host/browser run exposed missing SPA route fallback for
`/studio` and `/expert`; both hosts now serve the application routes correctly.
The original failure and corrected rerun remain in the ledger. A Windows attempt
to rebuild the running legacy server failed with a file lock; the later normal
browser launch rebuild succeeded. Regressions were fixed rather than waived.

The interactive browser plugin reported no connected browsers. Repository
Playwright tests and their screenshots provide the available visual evidence.
`studio-contract.png` shows a **Contract fixture** project, not the durable
Agentique self-model. On a real launch in this environment the operator sees the
Studio shell and missing-publication setup state.

## Resume the real product gate

Follow [the Studio guide](../../../docs/agentique-studio.md) with the two existing
authenticated cache paths. Launch against the retained self-model database if
available, or allow the host to seed a new durable repository from checked-in
SysML. Run the explicit accepted-cache Studio/view gates and the three precise
Phase 2 gates listed in `phase2-gates.md`. Then perform the requested operator
sequence and record actual latency and a real semantic screenshot.

```powershell
cargo test --locked --offline -p agq-studio durable_studio_candidate_view_and_history_vertical -- --ignored --nocapture
cargo test --locked --offline -p agq-modeling-view --test accepted_views -- --ignored --nocapture
```
