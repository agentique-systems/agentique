# Stages

The single progress record (ROADMAP §6, §8.3). One
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
  records carried into the governing text, now ROADMAP §7.5 (#26).
- Local leftovers `target-phase2-frontend/` and `test-results/` deleted.

**Measured**

- Tracked files: 5,941 at the archive tag, about 700 after. The `verification/`
  tree alone was about 3.6 GB in the working copy.
- Workspace tests: 1,117 passed, 0 failed, 13 ignored after the removals
  (1,254 with Native Studio in the workspace). CI takes about 17 minutes, most
  of it the language crates' tests.

- Not tried: building the archive tag (it needs about 15 GB of free disk,
  which this machine did not have during the run).

**Decided overnight** (ROADMAP §7.6):

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
- **Decision R-3: rebuild** (ROADMAP §7.2; decided overnight, pending
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
  states and actions (Stage 7).

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

Status: **provisionally complete, pending Operator acceptance.** (Its work
started on 2026-09-27 while the last Stage 1 review fixes were being made; the
Stage 2 builders did not depend on that code.)

**Done**

- The System State interface (#30), then hardened (#33): typed operations
  (create, delete, rename, move, connect, set property, lock, unlock) in atomic
  changes; rejections leave the model unchanged; a well-formed change that makes
  the model invalid is applied and its problems shown at the elements (R-18);
  locks cover what they own (R-11) and ask the Operator; undo/redo per change;
  change events; `undo_since` for undoing Assistant work (R-12). What can be
  written as SysML text is checked in one place in the language core.
- Git-backed History and `Project` (#32): embedded git, `model/` folder with
  `agentique.json` (identities, locks, next id), crash-safe continuous saving,
  checkpoints, branches, the "what changed" comparison between checkpoints.
- The Studio on the System State (#34): the Surface, Inspector,
  Requirements and History panels read the System State directly; direct
  manipulation for every operation, all in the command palette with shortcuts;
  lock confirmation; change highlighting; about 12,000 lines of the old
  engine's plumbing removed from the Studio.
- Visual quality pass (#31): design tokens, Inter and JetBrains Mono, Surface
  depth, change glow, readable labels.
- Pre-realignment engine retired (#35): the Generation 2 language
  engine, the modelling platform, the SQLite store, the generators and the
  conformance data are gone; the workspace has five crates.

**Measured** (URL shortener unless noted; Windows 10, release)

| | |
|---|---|
| Open a project | 3.3 ms |
| One edit applied, validated and saved (fsync) | about 6 ms |
| Checkpoint | 18 ms |
| "What changed" between checkpoints | 0.9 ms |
| One edit on a 2,048-element model (apply + validate + event) | 8 ms (55 ms debug) |

**Decided overnight** (ROADMAP §7.6, pending the Operator's confirmation):
R-18; the identity file format; deleted-target references re-bind by name
like a reload; saves write only changed documents; a project's repository is
used only if rooted at the project folder; Q-6: stay with egui.

**Q-6 (toolkit)**: stay with egui. A throwaway egui chat prototype streamed
Markdown with tool cards, element links, question prompts and stop/retry at
under 0.5 ms per frame for a normal conversation. What egui lacks (bold weight
in rich text, cross-block selection, inline widgets in wrapped text) is
covered by about two weeks of mitigations planned with the Conversation panel:
our own Markdown layout, a virtualised message list, copy buttons, and
accessibility labels. A web stack would split the app in two and put the GPU
Surface behind a webview.

**Not done or not tried**

- The Operator has not used it yet; the journeys are scripted UI input
  (`a-build` 74 steps, `a-crash`, `a-reopen` 14 steps, all passing).
- The light theme was not inspected.
- Branches: supported by `Project`, no UI yet. Merging branches by element
  identity is not built.
- The crash journey aborts the process after a completed save; saves
  interrupted mid-way are covered by History's own tests (every step). A real
  network share was not tried (only a simulated one).
- After reopening, a remembered layout position can leave a vertical gap.

**Operator: try this**

1. `cargo run --release -p agq-studio-native`, then **New project** in an empty
   folder.
2. Build the URL shortener by hand (A3): create parts (api, store,
   statistics), ports, connect two ports by dragging, rename with F2, set
   types and multiplicities in the Inspector, add a requirement and mark what
   satisfies it. Make a mistake on purpose (a type that does not exist) and
   see it reported at the element; undo it (Ctrl+Z).
3. Lock a part (L) and try to rename it (A4): a confirmation appears.
4. Checkpoint (Ctrl+S) with a message; make more changes; open the History
   panel and compare with the checkpoint.
5. Close the Studio and reopen the project (A9): everything as left,
   including the lock and the history. Kill it from Task Manager right after
   an edit and reopen.
6. Judge the look and feel for daily use (ROADMAP §3.1, C-21).

The same journey runs scripted (screenshots to a folder of your choice):

```text
cargo build -p agq-studio-native --features automation
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-session --scenario a-build --project %TEMP%\agq-demo --gallery %TEMP%\agq-shots
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-session --scenario a-crash --project %TEMP%\agq-demo
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-session --scenario a-reopen --project %TEMP%\agq-demo --gallery %TEMP%\agq-shots
```

## Stage 3: the Assistant

Status: **built, provisionally complete, pending Operator acceptance. The live
Claude API has not been tried**: there is no API key on the build machine, so
everything is tested with a scripted stand-in model. (Stage 3 work started on
2026-09-27 while the last Stage 2 changes were being merged; nothing of it
merged before Stage 2 did.)

**Done**

- The Assistant (#36, `crates/assistant`): tools over the System State
  (`read_model`, `find_elements`, `get_problems`, `apply_changes`,
  `ask_operator`). `apply_changes` resolves every name, tries every operation
  on a copy of the model and hands the Studio one change (one undo step); the
  Studio applies it exactly like an Operator edit, so locks ask first and the
  Surface highlights the change. The Claude API client streams with adaptive
  thinking, checks tool input itself, uses prompt caching and server-side
  refusal fallbacks, explains errors plainly, retries only before any output,
  and stops within about 50 ms. The tool-use loop runs on a background thread.
  Skills: the SysML subset, the slop rules, mapping ideas onto the
  architecture first, asking on major decisions, locks, using the tools.
- The Conversation panel (#37): a column in the Studio (Ctrl+J) with streamed
  Markdown replies (own renderer on `pulldown-cmark`, cached), element names
  as links that select on the Surface, tool calls as live cards (spinner,
  what changed, problems; expandable), questions as prompts with option
  buttons, Stop, Retry, edit-and-resend, Insert selection (Ctrl+I), a
  missing-key banner, and "Undo the Assistant's changes". The conversation is
  saved per project and restored on reopen.
- Tests without network: 49 Assistant tests (canned streams, a local HTTP
  stand-in, the loop on a real System State with a scripted model), 7 Studio
  conversation tests, and the `a-assistant` journey through the UI (19 steps):
  a scripted Assistant builds part of the URL shortener, asks whether click
  statistics is a separate part, an element link selects on the Surface,
  "add expiring links" hits the locked API and asks the Operator, Stop, and
  undoing the Assistant's changes.

**Decided overnight** (ROADMAP §7.6, pending the Operator's confirmation):
Q-9, the conversation is stored per project in the app's local data next to
the session file, not in the code repository; the default model is
`claude-opus-5` (`AGENTIQUE_MODEL`) at effort `high` (`AGENTIQUE_EFFORT`), with
server-side refusal fallbacks on; "Undo the Assistant's changes" undoes every
change since the turn started and is offered only while nothing has changed
since the turn ended.

**Not done or not tried**

- The live Claude API, including the small A1–A2 smoke test
  (`cargo run -p agq-assistant --example smoke` needs `ANTHROPIC_API_KEY`).
  Whether the real model follows the skills (asks on major decisions, maps
  "expiring links" onto the architecture first, keeps names plain) is
  therefore untested.
- The light theme; very long conversations; Markdown tables; text selection
  in replies (copy buttons instead); a virtualised message list.

**Operator: try this**

1. Set `ANTHROPIC_API_KEY` (optionally `AGENTIQUE_MODEL`), run
   `cargo run --release -p agq-studio-native` and create a new project.
2. Open the Conversation (Ctrl+J) and describe the URL shortener in ordinary
   words (A1). Watch the parts, ports, interfaces and requirements appear, each
   action as a card (A2); answer its questions; press Stop mid-way and undo the
   Assistant's changes.
3. Adjust by hand and in conversation (A3); lock the parts you consider settled
   (A4); ask for "expiring links" (A8) and check that it explains what would
   change and asks before touching a locked part.
4. Close and reopen (A9): model, locks, history and conversation as left.

Without a key, the scripted journey shows the same flow:

```text
cargo build -p agq-studio-native --features automation
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-session --scenario a-assistant --project %TEMP%\agq-assistant --gallery %TEMP%\agq-shots
```

## Stage 4: live proof and foundations

Status: **provisionally complete, pending Operator acceptance.** Built
overnight on 2026-09-27 under the Operator's overnight instructions (ROADMAP
§7.6): the evidence each gate asks for was produced and kept outside the
repository, and every choice that is the Operator's took ROADMAP's
recommendation and is recorded as pending confirmation. Stages 0–3 still wait
for the Operator's own live run (C-29, W4.4): nothing here says they were
accepted.

**Done** (PRs #39–#50)

- **W4.1** (#39): `REALIGNMENT.md` retired; everything points at `ROADMAP.md`.
  The Operator's amendments of 2026-09-27 are edited in: DeepSeek and Jev in
  C-35 (Q-11 resolved), the Jev clarification of C-34, DeepSeek and Jev
  columns in §4.8 with sources.
- **W4.8** (#40): the Providers part in the self-model; the architecture check
  fails when a crate outside Providers uses rig, tokio, reqwest or the
  credential store (`reqwest` in `agq-assistant` is the one temporary
  exception, until W5.7).
- **S4.2, kept as the start of W5.7** (#41, §7.6): `agq-providers` on rig
  0.42.0 (pinned exactly) for Anthropic, OpenAI (Responses API), OpenRouter
  and DeepSeek, through one generic code path; no rig types in its API; a
  synchronous handle that cancels at once; retries only before anything
  streamed; plain errors; native-tls (schannel on Windows).
- **The Assistant on the provider layer** (#42): with only `DEEPSEEK_API_KEY`
  set it runs `deepseek-flash` at effort `high` (C-35); `AGENTIQUE_PROVIDER`,
  `AGENTIQUE_MODEL` and `AGENTIQUE_EFFORT` choose otherwise. Anthropic keeps
  the hand-written client until W5.7.
- **W4.2** (#42, #43, #50): thinking shown as a collapsed row per step
  (R-31; Claude asks for `display: summarized`, DeepSeek shows its reasoning,
  with any quoted model text hidden, C-4);
  `read_model` returns an outline, and every tool result is capped at about
  8,000 tokens (R-34); the skills forbid showing SysML text (C-4).
- **W4.3** (#44): the evaluation set, 24 Scenario A tasks graded on the System
  State, with must-hold checks in every trial.
- **W4.4, headless part** (#46): Scenario A steps A1–A4, A8 and A9 live
  through the real turn loop on a project in git.
- **W4.7** (#45): CPU-side budgets in CI (release builds), the start time in
  the metrics report, budget assertions in the stress harness.
- **Interfaces for Stage 5**: the provider API (1, #41), the settings table
  and `settings.json` format 1 (2, #47), conversation format 2 as a
  specification (4, #48), the event protocol (5, #42), the budget report
  fields (6, #45). The tokens module and component list (3) wait for the
  toolkit upgrade (W5.1) and are the first Stage 5 item, before any fan-out.

**Live results** (DeepSeek `deepseek-flash`, effort `high`; reports and
transcripts outside the repository)

- **Evaluation set, final run** (24 tasks × 3 trials, after the review of the
  graders): every task passes every check in all three trials (pass^3 24/24,
  72 of 72 trials); must-hold failures 0; trials that did not run 0; about
  $0.34. Two earlier full runs with looser graders gave pass^3 22/24 and 23/24;
  every flag there was read by hand: three heuristic false positives of the
  claim check and one scripted follow-up that invited a change. The claim
  check remains a heuristic, and "asked before changing" is judged from tool
  calls and replies ending in a question.
- **Headless Scenario A** (A1–A4, A8, A9): 7 of 7 checks pass, $0.02. In A3
  the "requirement added in words" check passed trivially: A1 had already
  added the requirements. In A8 the one lock confirmation was refused; the
  Assistant modelled expiry on unlocked items and said so.
- **Seen live**: effort `high` accepted; reasoning streamed; tool round trips
  with `reasoning_content` sent back accepted; cache reads reported (4,608 of
  4,811 input tokens on a second call). Spend for the whole stage: about $1.05
  of DeepSeek (1,183 calls, logged at peak-hour prices).

**Measured** (reference run on this machine, release build of `main` at `4ceec616`, 2026-09-27
23:10–23:20, while two builders compiled in the background, so frame numbers
are pessimistic; raw reports outside the repository)

| Budget (§3.3) | Target | Measured | |
|---|---|---|---|
| Pan and zoom, 1k: interval p95 | ≤ 8.3 ms | pan 6.38, zoom 6.41 ms (UI CPU p95 1.9 ms, GPU pass p95 0.16 ms) | met |
| Input to next update, 1k: p95 | ≤ 8.3 ms | pan 5.75, zoom 5.87 ms | met |
| Pan and zoom, 10k: interval p95 | ≤ 16.7 ms | pan 23.8, zoom 21.1 ms (UI CPU p95 14.1 ms, GPU 2.9 ms) | not met (W5.5) |
| Input to next update, 10k: p95 | ≤ 16.7 ms | pan 25.3, zoom 19.6 ms | not met (W5.5) |
| Start to first update, warm | ≤ 400 ms | 462 and 489 ms (791 ms right after a build) | not met |
| Memory, start screen and 1k | ≤ 300 MB private | 351–373 MB | not met (R-44) |
| Memory, 10k | ≤ 450 MB private | 504–527 MB | not met (R-44) |
| Scene build (every edit), 1k / 10k | 50 / 100 ms | 105–130 ms / 2.56–3.34 s | not met (W5.5); CI guards regressions |
| System State edit, 2k / 10k | — / its share of 100 ms | 7.6 ms / 49 ms | checked in CI |

Journeys (release build with `automation`): `a-build`, `a-crash` (exits 3 by
design after its save), `a-reopen` and `a-assistant` pass; screenshots in
`overnight\shots\stage4\`.

The reference run, as run here (PowerShell, from the repository root, after
`cargo build --release -p agq-studio-native --features automation`):

```text
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --frames 2 --metrics <out>\start.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress1000 --scenario stress --gpu-timestamps --scenario-report <out>\stress-1k.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress10000 --scenario stress --gpu-timestamps --scenario-report <out>\stress-10k.json
```

The stress run exits 2 and names each budget it missed; at 10k it will until
W5.5. Memory is read from the process counters (not in-process yet).

**S4.2: rig parity** (decided overnight, pending the Operator's confirmation)

| Item | Result |
|---|---|
| P1 loop, tool and conversation tests; `a-assistant` journey against the new layer | pass; the scripted model is unchanged; `agq-providers` has its own tests against canned streams and a local server for all four providers (the Claude client's tests stay while it does, until W5.7) |
| P2 live on Anthropic | not tried: no key |
| P3 live on OpenAI and OpenRouter | not tried: no keys |
| P3a live on DeepSeek | works: the evaluation set and Scenario A above |
| P4 build | native-tls chosen (no C crypto library on Windows); rig adds about 0.7 GB of debug artefacts; the Studio's release rebuild took 2 min 38 s with `automation` while other builds ran, so the 30% rule is not measured cleanly |
| P5 no rig type in the public API | pass (reviewed) |
| P6 server-side fallbacks (Q-18) | a thin adapter in `agq-providers` (#49) removes Anthropic's `fallback` block before rig reads it and drops the declined model's reasoning and tool calls, as the hand-written client does; tested on canned streams split at every chunk size, not tried live; the upstream contribution is written as a proposal outside the repository, not filed |

Decision by its rule: proceed with rig for every provider (C-34). Anthropic
moves onto it in W5.7; P2 needs a live Anthropic key before the hand-written
client is removed.

**S4.1: toolkit** (provisional, pending the Operator: blind scoring and the
gates that need a person)

| Gate | Track A: egui 0.36.2, wgpu 30 (#54) |
|---|---|
| G1 toolkit cost | 1k pan and zoom p95 6.26–6.40 ms (UI CPU p95 2.1–2.4 ms): within 8.3 ms; 10k camera-only toolkit cost estimated at about 1.4 ms of CPU (steady frames minus the Surface's own work): within 2 ms. The 30-second run with every panel open and chat streaming, and the maximum frame, were not tried. Partially measured |
| G2 input to present | not measured; the proxy, input to next update, is 5.7 ms at 1k. Not tried |
| G3 chat | 200 messages, 59,800 words: scroll p95 6.23 ms, streaming p95 6.25 ms, first frame after loading 25.8 ms (in memory, not from disk). Works |
| G4 selection across messages | an automated journey drags across three messages while a reply streams in below and checks the copied text; a unit test checks the copy order. Works (automated); not tried by hand |
| G5 real weights | Inter's variable font at 400, 500 and 600, checked at 100%, 150% and 200% by a test and screenshots. Works; at 150% and 200% a 1600×1000 window is narrower than the Studio's minimum width, so the top bar's buttons overlap |
| G6 Narrator | needs the Operator; the automated proxy (seven AccessKit names and roles) passes |
| G7 Japanese IME | needs the Operator |
| G8 builds | clean release build 10 min 44 s; release rebuild 3 min 38 s; debug rebuild of the Studio 7–17 s; executable 29.9 MB (was 21.6 MB). Partially measured |

Track B (GPUI): **not tried**: under 12 GB of disk was free while Track A
built, and switching needs the Operator to accept the governance risk
(§7.6). Provisional decision: **stay with egui 0.36**. The rule cannot settle it
yet: G2, G6 and G7 were not tried for Track A and Track B was not tried at
all. Track A failed no gate it was measured on, and without Track B there is
nothing to switch to. Decided overnight 2026-09-28, pending the Operator's
scoring of text and motion and the gates that need a person.
Track A's code is kept as W5.1 (#54). egui 0.36 needs Rust 1.95, so the
toolchain moved to 1.97.1.

**Decided overnight** (ROADMAP §7.6 and §7.4, each pending the Operator's
confirmation): S4.2's code kept as the start of W5.7; native-tls; Anthropic
stays on the hand-written client until W5.7; Q-10 (app data), Q-17 (Inter),
Q-18 (a thin adapter); interim CI ceilings for budgets not met yet; conversation
format 2 specified rather than coded, since W5.7 is not fanned out; the tokens
interface moved to the start of Stage 5; Track B not tried (disk); the
thinking row hides SysML lines of reasoning (C-4, #50); S4.1 stays with egui
0.36 and keeps Track A as W5.1.

**Not done or not tried**

- The Operator's live run of §2.2 (W4.4) and the acceptance of Stages 0–3.
- The rubric grader for simplicity (R-19): it needs calibrating against the
  Operator.
- Live runs on Anthropic, OpenAI and OpenRouter.
- The 10k Surface, start and memory budgets (W5.5, R-44).
- The Jev thin client: C-35 names it, and W5.7 builds it with the key test of
  Settings (E4); Stage 7 uses it.

**Operator: try this** (the live acceptance of §2.2)

1. Set the DeepSeek key for this PowerShell session only (Settings, with the
   Credential Manager, is Stage 5), then start the Studio and create a fresh
   project (L1); the Conversation header shows `deepseek-flash · high`:

   ```text
   $env:DEEPSEEK_API_KEY = "<your key>"
   $env:AGENTIQUE_PROVIDER = "deepseek"
   cargo run --release -p agq-studio-native
   ```

2. L2–L3: describe the URL shortener in ordinary words (Ctrl+J opens the
   Conversation); watch parts appear, a collapsed thinking row between steps,
   tool cards; adjust by hand and in words.
3. L4: lock the settled parts (L), then "add expiring links"; refuse the lock
   prompt once.
4. L5: press Stop mid-turn, then "Undo the Assistant's changes".
5. L6: close and reopen; kill the process after an edit and reopen.
6. L7: the evaluation set on your key (about $0.35 a run at peak prices; the
   spend guard refuses a run whose worst case passes your stop):

   ```text
   $env:AGENTIQUE_SPEND_LOG = "$HOME\agentique-spend.jsonl"
   $env:AGENTIQUE_SPEND_STOP_USD = "2"
   cargo run --release -p agq-assistant --example eval -- --out $HOME\agentique-eval
   cargo run --release -p agq-assistant --example scenario_a -- $HOME\agentique-scenario-a
   ```

Then accept Stages 0–4 or list what fails (§2.2 L7).
