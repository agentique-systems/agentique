# Stages

The single progress record for the realignment (REALIGNMENT §5, §7.3). One
short section per stage: what was done, what was measured, what failed, what
is deferred, and what the Operator should try. A stage is complete only when
the Operator has used its outcome and accepts it (C-15).

Status values: **not started**, **in progress**, **provisionally complete,
pending Operator acceptance**, **accepted**.

## Before Stage 0: preserve the past

Status: done.

- Merged `platform/native-studio-alpha-acceptance` into `main` as is, with the
  previously untracked `verification/native-studio-acceptance/` records
  (PR #21, merge commit). Its old CI is red by design.
- Annotated tag `archive/pre-realignment` on `main` (C-22). Every retired file
  is recoverable from it.
- Full backup bundle outside the repository:
  `../agentique-pre-realignment.bundle`, checked with `git bundle verify`.
- Deleted the 22 local branches already merged into `main`. Pruned 61 worktree
  records whose folders no longer existed. Unmerged branches, the remaining
  worktrees and remote branches are left for the Operator.

## Stage 0: archive and clean

Status: **provisionally complete, pending Operator acceptance.**

**Done** (PRs #21–#28):

- Archive tag `archive/pre-realignment` and a verified bundle (above).
- One CI workflow (#23): fmt, clippy with warnings denied, workspace tests, the
  metamodel generator check and the architecture check. Green on a healthy tree.
- Removed Generation 1, the browser Studio, the HTTP adapters, the console and
  the browser tests (#24).
- Removed `verification/`, `docs/` (except this file), the ADRs, the
  conformance registers, `requirements.json`, `contracts/`, `scenarios/` and the
  retired tools (#25). Node is down to one dev dependency (R-17).
- Native Studio joined the root Cargo workspace with no dependency version
  change. Its evidence-generating harnesses are deleted; the UI driver and the
  pan/zoom stress run compile only with `--features automation` (#27). The
  production binary lost 29% of its source lines.
- `models/agentique/` rewritten as the realigned architecture in SysML, and
  `tools/check_architecture.py` fails CI when a crate dependency contradicts it
  (R-15, #28).
- `AGENTS.md` and `README.md` rewritten; still-valid decisions from the retired
  records carried into REALIGNMENT §6.6 (#26).
- Local leftovers `target-phase2-frontend/` and `test-results/` deleted.

**Measured**

- Tracked files: 5,941 at the archive tag, about 700 after. The `verification/`
  tree alone was about 3.6 GB in the working copy.
- Workspace tests: 1,117 passed, 0 failed, 13 ignored after the removals
  (1,254 with Native Studio in the workspace). CI takes about 17 minutes, most
  of it the language crates' tests.

- Not tried: building the archive tag (it needs about 15 GB of free disk,
  which this machine did not have during the run).

**Decided overnight** (see REALIGNMENT §6.7):

- Studio may depend on Assistant (the Conversation drives it); R-14 refined.
- R-15 uses SysML `dependency` between part definitions, listed explicitly
  (not transitive), with temporary dependencies marked in the model. Four
  temporary dependencies exist, all from the SQLite revision store that Stage 2
  replaces.

**Deferred**

- Renaming the Studio's "World" navigation: the Studio's structure is reworked
  in Stage 2.
- Rewriting the Node pinning tools in Python to drop Node entirely (R-17):
  feasible (five small files) but not needed until they are run again.
- `crates/runtime-publications`, the SQLite store and the runtime bundle stay
  until Stages 1–2 replace them.

**Inventory after Stage 0**

| Folder or crate | Purpose |
|---|---|
| `crates/kernel` | Generic element graph and identities (LanguageCore) |
| `crates/kerml`, `crates/sysml` | KerML/SysML metamodel descriptors generated from the pinned XMI |
| `crates/kerml-syntax` | Lossless, error-tolerant parser of KerML/SysML text |
| `crates/kerml-text` | Lowering text into the element graph, source projects and identity reconciliation |
| `crates/kerml-semantics`, `crates/sysml-semantics` | Name resolution, typing, inheritance and validation queries (Stage 1 decides extract or rebuild) |
| `crates/standard-libraries` | Loads the pinned standard library sources |
| `crates/runtime-publications` | Loads the precomputed library bundle; retired in Stage 1 |
| `crates/modeling-workspace`, `modeling-service`, `modeling-view` | Current live model state, edit service and view queries (become SystemState in Stage 2) |
| `crates/modeling-repository`, `adapters/modeling-sqlite` | Current SQLite revision store (replaced by git in Stage 2) |
| `crates/modeling-agent` | Current agent types (become the Assistant in Stage 3) |
| `crates/studio-native` | The Studio application (egui + wgpu) |
| `crates/studio-scene` | Surface layout, level of detail and hit testing |
| `crates/studio-platform` | Connects the Studio to the modeling service and runtime bundle |
| `tools/metamodel-gen` | Generates the descriptors in `crates/kerml` and `crates/sysml` |
| `tools/kerml-grammar`, `tools/sysml-grammar` | Generate the parser tables in `crates/kerml-syntax` |
| `tools/*.mjs` | Acquire and check the pinned standards artifacts (run rarely, by hand) |
| `tools/runtime-recovery`, `tools/*runtime*.py` | Rebuild the runtime bundle; retired with it in Stage 1 |
| `tools/check_architecture.py` | The R-15 architecture check |
| `models/agentique/` | Agentique's architecture in SysML |
| `standards/` | Pinned specifications, libraries, grammar, generated descriptors |

**Operator: try this**

1. `git checkout main && cargo build --workspace && cargo test --workspace`
   (expect all green) and `python tools/check_architecture.py` (expect "OK").
2. Read `README.md`, `AGENTS.md` and `models/agentique/Agentique.sysml`; check
   you can say what each folder and crate above is for.
3. Spot-check the archive: `git worktree add ../agq-archive archive/pre-realignment`
   then `cargo build --workspace` there (old state builds), and remove the
   worktree afterwards.
4. Open the Studio: `cargo run --release -p agq-studio-native`.

## Stage 1: fast, honest language core

Status: **provisionally complete, pending Operator acceptance.**

**Done**

- Two review rounds found six blocking problems (a crash on import cycles,
  references bound by name, poor parser recovery, comments swallowing
  members, printed names that could re-bind after reload, an import cache
  keyed by the wrong thing) plus smaller ones; all were fixed before merging.
  A valid model now always prints text that reloads with the same bindings.
- Three spikes ran in parallel (all on 2026-09-27; they started while the last
  Stage 0 pull requests were in CI, after every Stage 0 change was finished and
  locally green):
  - *extract*: measured the retained Generation 2 engine without the runtime
    bundle, with producer closure, publication and audits bypassed;
  - *rebuild*: wrote a small subset core, `crates/language` (`agq-language`);
  - *git*: prototyped git persistence, `agq-history`, on its own branch (used
    in Stage 2).
- **Decision R-3: rebuild** (REALIGNMENT §6.2, decided overnight, pending
  Operator confirmation). `agq-language` is the language core: about 3,800
  lines, no dependencies, one generic element tree whose references carry
  their target's identity, with `parse`, `print` and
  `validate`. See `crates/language/README.md`.
- The URL shortener model: `models/url-shortener/UrlShortener.sysml` (API,
  link store, click statistics, two interfaces and one connection between
  them, two requirements with `satisfy`). It validates with no diagnostics.
- Subset manifest `docs/subset.md` (R-8) and deviations list
  `docs/deviations.md` (R-9, eleven entries; entries 7–11 are stricter than
  the standard and need the Operator's confirmation).

**Measured** (Windows 10, single runs, URL shortener: 115 lines, 78 elements)

| | debug | release |
|---|---|---|
| parse (with linking references) | 0.4 ms | 0.29 ms |
| validate | 1.2 ms | 0.32 ms |
| rename a part + re-validate + print | 1.1 ms | 0.73 ms |
| 20 copies (2,300 lines): validate / edit | 22 / 22 ms | 3.8 / 6.0 ms |

The Generation 2 engine on the same model, bundle and certification bypassed
(release build): loading the standard library from the pinned sources took
266–303 s (11 fixed-point rounds over 36,731 elements, about 950 MB), the model
another 70 s, so 5.5 minutes to the first validated view; every edit took about
72 s (1.6 GB peak), because each edit rebuilds the library and model graph and
re-resolves everything. Conjugated port typing (`~Port`) failed with an internal
error instead of an "unsupported" report. Removing the certification machinery
(about 18k lines) would leave a 23–27k-line core that is still not live. That
settled R-3. The Generation 2 parser alone took 24–32 ms (release) on this
model and rejected two forms the specification's own examples use.

Git persistence (spike, release): reload of the URL shortener 0.3 ms from the
working folder and from a commit, save + commit 37 ms; 2,000 elements in 20
files: reload 2–3 ms, save + commit 69 ms (parsing excluded).

**Not done / deferred**

- `crates/runtime-publications` and the Generation 2 crates are not removed
  yet: the current Studio still runs on them. Stage 2 moves the Studio onto the
  System State and then retires them all (R-3).
- Printed names are correct but not yet the shortest possible after a rename
  (they fall back to the qualified name).
- Not supported yet (reported explicitly as unsupported): expressions (only
  literal values), n-ary connections, feature chains as `satisfy` targets,
  states and actions (Stage 4).

**Operator: try this**

```text
cargo run -p agq-language --example check -- models/url-shortener
cargo run -p agq-language --example check -- crates/language/tests/fixtures
```

The first prints `valid` with timings. The second reports the deliberate error
(`error[incompatible-ends]`) at `UrlShortener::UrlShortenerService::clickReporting`,
line 96 of the broken copy, and exits with code 1. Try your own mistake: copy
`models/url-shortener/UrlShortener.sysml`, change a type name or remove a `~`,
and run the check on the copy. Please also read `docs/deviations.md` entries
7–11 and say whether the stricter rules are what you want.

## Stage 2: Studio foundation

Status: not started.

## Stage 3: the Assistant

Status: not started.
