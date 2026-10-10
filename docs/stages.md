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
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress1000 --scenario stress --scenario-report <out>\stress-1k.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress10000 --scenario stress --scenario-report <out>\stress-10k.json
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

**S4.1: toolkit** (superseded: the Operator chose GPUI, C-48, and waived the
remaining gates; the record below is kept as it was)

| Gate | Track A: egui 0.36.2, wgpu 30 (#54) |
|---|---|
| G1 toolkit cost | 1k pan and zoom p95 6.26–6.40 ms (UI CPU p95 2.1–2.4 ms): within 8.3 ms; 10k camera-only toolkit cost estimated at about 1.4 ms of CPU (steady frames minus the Surface's own work): within 2 ms. The 30-second run with every panel open and chat streaming, and the maximum frame, were not tried. Partially measured |
| G2 input to present | not measured; the proxy, input to next update, is 5.7 ms at 1k. Not tried |
| G3 chat | 200 messages, 59,800 words: scroll p95 6.23 ms, streaming p95 6.25 ms, first frame after loading 25.8 ms (in memory, not from disk). Works |
| G4 selection across messages | an automated journey drags across three messages while a reply streams in below and checks the copied text; a unit test checks the copy order. Works (automated); not tried by hand |
| G5 real weights | Inter's variable font at 400, 500 and 600, checked at 100%, 150% and 200% by a test and screenshots. Works; at 150% and 200% a 1600×1000 window is narrower than the Studio's minimum width, so the top bar's buttons overlapped (fixed in #82: the top bar drops what does not fit) |
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

## Stage 5: the daily-use Studio and Settings

Status: **in progress.** Built overnight on 2026-09-27/28 under the Operator's
overnight instructions (ROADMAP §7.6), after Stage 4 was recorded as
provisionally complete. Several work items have not started (below), and the
acceptance needs the Operator: a week of real use, Scenario E with real keys
for every provider, and the side-by-side review (§3.1). Nothing here says the
Operator accepted anything.

**Done** (PRs #52–#82)

- **W5.1 Toolkit, on GPUI** (C-48, the Operator's decision; this replaces
  #54, S4.1's Track A on egui 0.36): the Studio is rewritten on GPUI
  (`gpui-pre =0.3.7` with the unstyled `gpui-base =0.7.0`) and its
  presentation redesigned; egui, eframe and wgpu are gone. The state is one
  toolkit-free entity (`studio.rs`) that views follow by what changed; the
  Surface paints with GPUI's quads, paths and text by level of detail; the
  Panels, Conversation, palette, dialogs, welcome and Settings are built on
  one design system (`ui/`) with springs that reduced motion turns off;
  `--fixture components` shows every token and component, the real Surface
  and the real Conversation. Inter ships as three static weights instanced
  from Inter Variable (GPUI's Windows text system does not select a variable
  font's weights). Results on the reference machine (release, 165 Hz):
  start to first paint 251 ms warm; 1k pan and zoom p95 6.3 and 6.2 ms; 10k
  13.1 and 12.6–12.7 ms over two runs; the chat benchmark scrolls at
  p95 6.3 ms and streams at 6.6 ms. All six journeys pass (`a-build`,
  `a-crash` exits 3 by design, `a-reopen`, `a-assistant`, `d-daily`,
  `e-settings`), in light and dark. Screen-reader names use GPUI's AccessKit
  roles; with NVDA or Narrator: not tried. Waits for the Operator's use.
- **Interface 3** (#55): `tokens.rs` (every size, radius, stroke, duration and
  curve of §3.2; colour scales generated in OKLCH from base hue, accent hue and
  contrast for dark, light and high contrast, tested for 4.5:1 body text, 3:1
  lines on panel backgrounds and 4.5:1 text on solid colours), the component
  list of §3.2 with variants and states, and the gallery
  (`--fixture components`). The Studio's drawing moves onto the tokens in W5.2.
- **S5.1 and W5.5, first part** (#57, #59): an edit lays out the cards again
  with the layout memory and routes only the edges it touches
  (`Scene::update`); the Studio uses it. The spatial index is a growing dense
  grid.
- **W5.7 Providers, part** (#52, #58): TypeSafe AI's Jev through a thin
  client in `agq-providers` (typed decisions, prices, a key test); conversation
  format 2 (provider-neutral entries, each reply with its model; reasoning goes
  back only to the model that wrote it; unknown entry kinds kept and never
  sent), per project at `%APPDATA%\Agentique\projects\<folder>-<hash>\`, a
  Stage 4 conversation read once as a read-only transcript and left in place.
- **W5.8 Settings** (#53, #56, #60): keys in the Windows Credential Manager
  (tested before saving, never shown again, only a hint), key tests that run
  no model, model lists with capabilities and list prices; the Settings view
  (Ctrl+,) in place of the Surface with Providers, Assistant, Appearance,
  Keyboard and About, and search over synonyms; the Assistant's model from
  Settings (environment variables win); appearance lives in `settings.json`
  (a Stage 4 session hands its theme over once) and "Follow Windows" follows
  the Windows theme; estimated cost per turn and per day beside the model.
- **W5.4 and W5.11, part** (#62, #68, #71): Shift+1 fits, Shift+2 zooms to
  the selection, Shift+0 goes to 100% (by the key's place, whatever it types),
  + and - zoom, a zoom control in the Surface's corner, Space+drag pans even
  from a card, Ctrl+P goes to an element, ? lists every shortcut, the palette
  lists recent commands first, and the "no key" banner opens Settings ›
  Providers.
- **W5.3, part** (#67, #72): focus mode (`Ctrl+\`) gives the Surface the whole
  window; Ctrl+B, Ctrl+Alt+B and Ctrl+J collapse the Outline, the Inspector
  column and the Conversation; each project remembers its layout.
- **W5.5, second part** (#70): the GPU batch covers a margin around the view
  and is reused while the view stays inside it, so panning and zooming in no
  longer rebuild 114,560 instances a frame at 10k: 10k pan and zoom now meet
  C-33.
- **W5.6, part** (#69, #74): Markdown tables in the Conversation (copied as
  tab-separated text), "Copy" under each reply, and a model picker in the
  Conversation's header (the providers with a key; the choice is Settings').
- **W5.4, more** (#79): a minimap in the Surface's corner when the model does
  not fit in the view; click or drag it to move the view.
- **W5.8, more** (#77, #78): Settings' Projects section (the folder for new
  projects, recent projects with "Remove from the list", where conversations
  are kept) and Advanced section (the settings file; a Danger zone whose
  "Reset all settings" asks first and keeps `settings.json.bak`); Ctrl+,
  reopens at the last section viewed; search marks its words; rows an
  environment variable decides are shown disabled with "Set by …"; a key
  that works lists its models at once (#81, E2).
- **UI scale** (#82): at 150% and 200% the top bar keeps only what fits (the
  rest is in the palette), so nothing overlaps (S4.1's G5 finding).
- **W5.10 and W5.11, more** (#75, #76): the arrow keys move the selection
  between cards (the selection ring is the focus); the taskbar flashes when
  the Assistant stops while the window is in the background.
- **W5.9, part** (#63): the first run's three-step welcome, with the URL
  shortener as a sample project.
- **W5.12** (#61, #66): the `e-settings` journey (Scenario E without keys or
  the network) and `d-daily` (Scenario D from the welcome with the sample:
  fit, zoom to selection, go to element, the views, the shortcut list, focus
  mode, a checkpoint). `d-daily`'s screenshots found three bugs, fixed: the
  sample opened with a stale camera, its status called it "edited outside
  Agentique", and a long status ran over the status bar's counts.
- **W5.10, part** (#64): Settings' choices are labelled by their rows for
  screen readers.
- **W5.13 Library** (C-49, Scenario H; branch `stage5/library`): reusable
  building blocks that are ordinary KerML/SysML definitions. `agq-library`
  holds 30 built-in definitions in six packages (messages, interfaces,
  services, storage, messaging, resilience), indexes them with the project's
  own definitions and My Library (`library\My Library.sysml` beside the
  session file), searches them (fuzzy names, then qualified names and doc
  words) and plans their use as one System State change: a usage typed by the
  block, and a copy of what it needs in the project's `Library` package at the
  same qualified names, reusing identical definitions and never overwriting
  different ones (use the project's, or copy under another name), so a project
  stands alone. Where a copy came from is derived, never stored. "What can
  connect here?" uses the language's own port rule, exposed with its lookup as
  the read-only `Semantics` (the one locked-core addition, recorded in §7.6).
  The Studio has the Library tab beside the Outline (Ctrl+Shift+L: search,
  scopes, kinds, a structural preview), insertion by Enter, double-click, drag
  (onto a port it connects), Shift+A and the context menu; opening a
  definition with a breadcrumb and Back, inherited parts dashed and overrides
  marked; Find usages; the Inspector's Definition section with Override and
  Reset per inherited value; Specialise; the shared-definition question
  offering "Specialise instead"; Create building block from selection (ports
  where connections cross); Save to My Library. The Assistant has
  `search_library`, `read_library_block`, `use_library_block` and
  `save_to_library` (only when asked, after the Operator confirms), a skill
  section on reuse, block links in its replies, and five evaluation tasks that
  tell appropriate reuse from blind reuse (h8-*; "never saves to My Library
  unasked" is a must-hold). Tests: 26 in `agq-library` (search, closure, reuse
  of identical copies, conflicts, nested composites, specialisation,
  overrides, My Library, malformed and missing entries, locks, undo and redo,
  5,000-block search), 6 tool tests, 2 Conversation tests and 9 Studio tests.
  The `h-library` journey (75 steps: H1–H8, including a second project, the
  scripted Assistant reusing a block and modelling a plain definition when
  none fits, and undoing its changes) passes at 100%, 150% and 200% UI scale
  with reduced motion; the gallery shows the Library panel, previews, drag
  ghost, breadcrumb, inherited values and inherited and override cards. All
  journeys pass on debug builds with `automation` (`a-build`, `a-crash` exits
  3 by design, `a-reopen`, `a-assistant`, `d-daily`, `e-settings`; `d-daily`
  also at 150% and 200%). Library search measured 2.4 ms per keystroke at
  5,000 blocks (release; budget 8 ms). The reference budget run (§8.6: start,
  1k and 10k pan and zoom, chat, before and after) has not been run: the
  release build was stopped because the machine ran low on memory. Waits for
  the Operator's use.

**Live results** (DeepSeek `deepseek-flash`, effort `high`; reports outside
the repository)

- **Evaluation set** (24 tasks × 3 trials) after this stage's changes
  (conversation format 2, Settings, the model from Settings): pass@3 24/24,
  pass^3 24/24, must-hold failures 0, trials not run 0, about $0.34.
- **Headless Scenario A** on conversation format 2 (A1–A4, A8, A9, which
  saves, reopens and compares the conversation): 7 of 7 checks pass, twice
  (before and after the review fixes), about $0.02 each; the saved file is
  format 2 and every reply names `deepseek/deepseek-flash`.
- **Jev**: one live call, 349 ms, $0.000016 (#52).
- **Keys**: the DeepSeek key test answered "works" with no model run (#53).
- Spend for the night so far: about $1.44 of DeepSeek logged (at peak-hour prices; hard stop $16) and $0.000016 of Jev (hard stop $0.50).

**Measured** (reference machine: Windows 10, RTX 3060 Ti, Vulkan; release
builds; single runs; raw reports outside the repository)

| Budget (§3.3) | Target | Measured | |
|---|---|---|---|
| Pan and zoom, 1k: interval p95 | ≤ 8.3 ms | pan 6.18, zoom 6.22 ms (UI CPU p95 1.9 ms) | met |
| Input to next update, 1k: p95 | ≤ 8.3 ms | pan 5.57, zoom 5.59 ms | met |
| Pan and zoom, 10k: interval p95 | ≤ 16.7 ms | pan 6.28, zoom 6.33 ms (UI CPU p95 4.1 ms, GPU 1.7 ms; before #70: 20.2 and 19.0 ms) | met |
| Input to next update, 10k: p95 | ≤ 16.7 ms | pan 4.98, zoom 5.08 ms (before #70: 18.8 and 17.5 ms) | met |
| Edit to Surface, 10k (CPU, before presenting) | ≤ 100 ms | 127 ms median at 10,204 elements (scene 17 ms) | not met (W5.5) |
| Edit, Surface half, 1k / 10k (CI) | 50 / 100 ms | 5–7 / 71–86 ms (77.7 ms on CI) | met |
| Full scene build when a project opens, 1k / 10k | — / 1 s | 88 ms / 1.9 s | 10k not met |
| Start to first update, warm | ≤ 400 ms | 484–500 ms (no change with the Credential Manager reads skipped) | not met |
| Memory, start screen / 1k | ≤ 300 MB private | 379–414 / 424 MB between runs (Stage 4: 351–373 MB); DX12 instead of Vulkan: 384 MB | not met (R-44) |
| Memory, 10k | ≤ 450 MB private | 479 MB (Stage 4: 504–527 MB) | not met (R-44) |

The final run on `main` at `8515d472` (end of the night) confirms the 10k
rows: pan 6.26, zoom 6.28 ms p95, input to next update 4.94 and 4.99 ms; 1k
pan 6.18, zoom 6.23 ms; the stress harness passes every budget at 1k and 10k. Journeys: `a-build`, `a-crash` (exits 3 by
design), `a-reopen`, `a-assistant`, `d-daily` and `e-settings` pass (debug
builds with `automation`).

**Decided overnight** (ROADMAP §7.6, each pending the Operator's
confirmation): S4.1 stays with egui 0.36 and Track A becomes W5.1; S5.1's
design (full layout, incremental routing, kept detours until a full build)
and the R-28 amendment; the spike's code kept as the start of W5.5;
appearance moves from the session to `settings.json`, handed over once;
"per day" costs use the UTC day (no date library; the local time zone would
need a Windows call); the welcome shows until a first project is opened.

**Not done or not tried**

- **W5.2 Design system**: the Studio still draws with `theme.rs`, not the
  generated colours; no icon set (the licence check waits); typography side
  by side (Q-17 kept Inter); the literal-value check of §8.5 rule 1.
- **W5.3 Shell**: a docking API and panel widths remembered per project
  (egui keeps them for the session).
- **W5.4 Surface**: new cards (sized to content, badges), arrowheads by kind,
  label pills that never overlap containers, change marks by actor (a colour
  decision: the "changed" magenta and the Assistant's violet are close),
  named level-of-detail tiers. The dot grid, hollow and filled ports and the
  selection ring exist from earlier stages.
- **W5.5 Performance**: an edit on 10,204 elements takes 127 ms end to end
  (the scene 17 ms; applying and saving the change and rebuilding the scene
  input about 95 ms), over C-33's 100 ms; faster apply and save would touch
  the System State (the locked core, R-16): the Operator's call. The warm
  start (482 ms) and memory (R-44) miss their budgets; memory needs profiling.
- **W5.6 Conversation**: new tool cards, context chips in the composer, the
  other message actions.
- **W5.7**: Anthropic moves onto rig only after a live Anthropic run (P2); the
  hand-written client and the `reqwest` exception stay. OpenAI and OpenRouter
  not tried live (no keys).
- **W5.8**: deep links from other errors (the no-key banner has one); search
  does not reach the Projects row or the provider cards.
- **W5.10 Accessibility**: Narrator (G6) and Japanese IME (G7) need the
  Operator; the §3.5 checklist is not worked through yet.
- **W5.13**: the five h8 evaluation tasks have not run live (no spend was
  agreed for this work); screen readers on the Library (NVDA, Narrator) not
  tried; the Library's search and preview have not been judged by the
  Operator. Journeys that start from the welcome (`d-daily`, `h-library`)
  now scroll its buttons into view, so they also run at 150% and 200%.
- At 200% on a 1600-pixel window the three docked columns were wider than
  the window and the Surface was 0 pixels wide (at 150% it was 148 pixels).
  The columns now narrow so the Surface keeps a quarter of the window: first
  each gives up its width above the 200-point minimum, in proportion, then
  they share equally (at 100% on that window nothing changes; the widths
  kept for each project are unchanged). Tabs, segmented choices and the
  composer's selection chip now end in an ellipsis instead of overlapping
  when narrow. Dragging a splitter while the columns are narrowed sets the
  kept width, so the splitter does not follow the pointer exactly until
  there is room. Decided in this session (§7.6), pending the Operator.
- `--screenshot` and `--frames` runs now end the process once their image
  and report are written: GPUI's quit waited for an empty message queue,
  which the gallery (redrawn every tick in a debug build) delayed for a
  minute while saving the image again each tick.
- A shared build folder across git worktrees reused another worktree's build
  once tonight (Cargo judges freshness by file times); the S5.1 measurement
  was redone after touching the sources.

**Operator: try this**

1. See the first run, then add a key in Settings (E1–E3). A fresh session file
   shows the welcome:

   ```text
   cargo run --release -p agq-studio-native -- --session $env:TEMP\agq-fresh\session.json
   ```

   Press Ctrl+, › Providers › DeepSeek: paste the key, Test, Save. The
   Conversation header then shows `deepseek-flash · high` and, after a turn,
   its estimated cost and today's total.
2. Try "Start from the URL shortener", then Shift+1, Shift+2 on a selected
   card, + and -, Space+drag, the arrow keys, Ctrl+P and a name, ?,
   `Ctrl+\` (focus mode), Ctrl+B and Ctrl+Alt+B, and the minimap.
3. At 10k: `--fixture stress10000`, then pan and zoom (now within budget).
4. The design tokens and generated themes: `--fixture components`.
5. E4: test the Anthropic, OpenAI, OpenRouter and TypeSafe AI keys in
   Settings. An Anthropic key saved there is not used by the Assistant until
   W5.7 (set `ANTHROPIC_API_KEY` for now).
6. Confirm or change the overnight decisions above (§7.6).
7. The Library (Scenario H): open the URL shortener sample, press
   Ctrl+Shift+L, type "cache", look at CachedStore's preview and press Enter;
   select a port and use "What can connect here?"; press Enter on a composite
   usage to open its definition and Backspace to return; specialise it and
   override a value in the Inspector; select two parts and "Create building
   block from selection"; "Save to My Library…", then use it in another
   project. The scripted run:

   ```text
   cargo build -p agq-studio-native --features automation
   target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-h\session.json --scenario h-library --project %TEMP%\agq-h\demo --gallery %TEMP%\agq-h\shots
   ```

## Stages 7–8: the factory loop, with agents (C-50)

Status: **in progress.** Built on 2026-09-30 and 2026-10-01 in one session
under the Operator's direction of 2026-09-30 (C-50), on the branch
`stage7-8/factory-loop` (not merged). The milestones M1–M4 work end to end in
the tests, the journeys and two live runs; nothing here says the Operator
accepted anything. The decisions taken while building are in ROADMAP §7.6
(2026-09-30 and 2026-10-01), each pending the Operator.

**Built**, by work item

- **W6.9 Agents in the language** (`491c819a`): `enum def` and values, the
  built-in `Agents` library (`AgentMode`, `AgentOutput`, `Agent` with mode,
  model, `minConfidence`, `maxLatencyMs`, `maxCostPerCallUsd`, `fallback`),
  `wrong-fallback` and `agent-fallback`, deviation 12, the subset manifest.
- **W6.10, part**: agents are marked `agent · <mode>` on the Surface; the
  Inspector's Agent section lists the settings with what is inherited, and
  says what counts as a failure. The self-model validates under our own core
  and its crates are checked against it (`tests/dogfood.rs`).
- **W7.1** (`491c819a`): expressions as identity-linked references, the
  behaviour subset (exhibited state machines, transitions with triggers,
  guards and effects, send, assign, if, accept, time), verification defs as
  scenarios, the `Scenarios` library (`Outcome`, `StandIn`), System State
  properties for each new field; `parse_expression` for the controls and
  tools that write expressions.
- **W7.2** (`b0f898e0`, `4f167a9f`): `agq-simulation` compiles a scenario
  into disposable structures and runs it in logical milliseconds, with
  limits, cancellation, a trace, checks with reasons and results that keep
  completion, verdicts (passed, failed, not run, unsupported, blocked,
  inconclusive), provenance and freshness apart. The Library's blocks gain
  behaviour: `Resilience::RetryingWorker` and `Moderation` (an agent with a
  deterministic fallback), each with scenarios that come with the block
  when it is used, and pass.
- **W7.3** (`b0f898e0`, `241b06fb`): stand-ins with injected failures
  (timeout, invalid output, refusal, tool unavailable), the agent's contract
  (type, values, confidence in [0, 1], `minConfidence`, `maxLatencyMs`) with
  the fallback on failure; recordings keyed by the SHA-256 of the canonical
  request (keys sorted whatever the build), replay that stops at
  `missing-recording` and never calls a model; live evaluation through an
  explicit model client after the Operator confirms provider, model, calls
  and cost: samples, per-check counts with 95% Wilson intervals, failure
  categories, median latency, cost and provenance; "Keep as recordings".
- **W7.4** (`acbef1d8`): the Scenarios tab (Ctrl+Shift+R) with the newest
  result per mode and whether it is current; the Run panel: modes, Run (F5)
  and Stop (Shift+F5), what happened apart from the checks, verdicts with
  icon and colour, live and code provenance, a virtualised trace with
  filters, stepping (`[`, `]`), playback (`\`) and follow; results that no
  longer describe the model (worked out again on every change, undo
  included) are marked outdated and never drawn. The Surface marks where the
  trace is, where it has been and where it failed, and drift, by shape as
  well as colour. Scenarios are written through controls (what goes in,
  what to wait for, stand-ins, checks, time) and the Inspector (values may
  be expressions; guards; the Behaviour, Stand-in, Agent, Evidence and
  Implementation sections). Stand-ins are not architecture cards.
- **W8.1** (`a60ed049`): `agq-execution`: scopes (canonical paths; `..`,
  absolute, drive, UNC, streams and device names refused; writable and
  protected paths), Cargo-only commands (build, check, test, run,
  metadata), an environment without keys, tokens or secrets, offline unless
  allowed, process-tree kill, git worktrees, patches and integration, jobs
  with journals and recovery. Nothing runs until the Operator allows
  trusted-local execution for the project, in a dialog that says it is not
  a sandbox.
- **W8.2** (`a60ed049`, `4f167a9f`): `model/links.json` by element identity;
  checks with their coverage: module boundaries (the model's dependencies
  and a part def's own parts), crate boundaries (dogfood), contract shapes,
  linked tests; failing checks are drift at the elements, in the Inspector
  and Problems; the harness protocol and the implementation runner (the code
  is recorded once the harness is built).
- **W8.3** (`df0c4388`, `6860feb8`): the Assistant writes behaviour and
  scenarios with `apply_changes`, reads them (`inspect_behaviour`,
  `list_scenarios`) and asks the Studio to run them and read results, code
  links and checks, or to start an implementation task
  (`propose_implementation`, which the Operator starts or declines). The
  supervised loop: a brief from the model, a worker (the Assistant's loop
  with code tools) in its own worktree, protected paths, bounded repair
  (6 rounds, stop after 3 without progress), contract changes back to the
  Operator, the Studio's own verification, a review with the patch, the
  elements it touches, the proposed links, the worker's words and its cost;
  integration only if the repository has not moved; continuing a task in
  its worktree; interrupted tasks found on open.
- **W8.4** (`4f167a9f`, `241b06fb`, `87e98c32`): the URL shortener with AI
  screening implemented from its model in a repository of its own (a
  fixture, also the sample "And its code"); the proof test: boundaries, 13
  contract shapes and 3 linked tests pass, the six scenarios with stand-ins
  pass against the code, the agent's evaluation cases stop honestly (the
  harness calls no model), a held link going live is caught by the three
  scenarios that guard review and as drift at `reviewBeforeActivation`, and
  repaired, and a module boundary the model does not allow is found at
  `LinkApi`. The second example (the retrying dispatcher) runs the same
  runner and adapter, with its model and implementation each broken on
  purpose. The dogfood check runs on this repository.

**Checks** (on the branch at the commit of this record; debug builds)

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace
  --all-targets -- -D warnings`: clean, also for the Studio with
  `automation`. `python tools/check_architecture.py`: OK, 11 crates in 10
  parts, 23 allowed dependencies.
- `cargo test --workspace`: 503 pass, none fail; 8 ignored (6 budget and
  performance tests that run in release builds, and the 2 live tests).
- Journeys (debug, `automation`): `i-scenarios` (53 steps: scenarios, a model
  run and its trace on the Surface, a changed setting that outdates the
  result and makes its check fail, undone; a scenario written through
  controls; a walkthrough; code asking for trust; live asking first and
  cancelled) and `i-code` (14 steps: the sample with its code, trust in its
  dialog, a scenario through the real code, checks without drift), each at
  100%, 150% and 200% and with reduced motion. `a-build`, `a-crash` (exits
  3 by design), `a-reopen`, `a-assistant`, `d-daily` (also at 200%),
  `e-settings` and `h-library` (100%, 150%, 200%) pass. The 200% runs found
  that a dialog taller than the window hid its buttons: dialogs now fit the
  window and their body scrolls; "Create building block" asks for its names
  before its lists.

**Measured** (reference machine, release builds with `automation`, raw
reports outside the repository)

| Budget (§3.3) | Target | This branch | `main` at `68d529ea` (before) |
|---|---|---|---|
| Start to first update, warm | ≤ 400 ms | 276, 246 ms | — |
| Pan and zoom, 1k: interval p95 | ≤ 8.3 ms | pan 6.22–6.29, zoom 6.22 ms | — |
| Input to next update, 1k: p95 | ≤ 8.3 ms | 0.59–0.65 ms | — |
| Pan and zoom, 10k: interval p95 | ≤ 16.7 ms | pan 13.2–15.7, zoom 12.3–13.3 ms | pan 13.2–13.6, zoom 12.4–13.0 ms |
| UI CPU p95 / Surface paint median, 10k | — | 8.7–9.3 / 7.5–7.7 ms | 8.7–8.8 / 7.3–7.5 ms |

Every budget is met. The branch is within about 3% of `main` in paint time
at 10k, about the spread between runs; skipping the Surface's run and drift
marks when there are none changed nothing measurable. At 10k both are about
twice Stage 5's recorded 6.3 ms: that change is on `main` already (the
Library's merge, never measured, or the machine today), not in this branch;
it waits for W5.5. Memory was not measured.

**Live results** (DeepSeek `deepseek-flash`; reports outside the repository)

- **I3, evaluation of the screening agent** (`ScreeningCases`, 4 cases × 5
  samples, as modelled, `maxLatencyMs` 500): every answer took 0.9–5 s
  (median about 1.8 s), so every call was a timeout and the fallback
  decided: the scam and lookalike cases pass 5/5 (held), the encyclopedia
  and documentation cases fail 0/5 (held instead of allowed). With
  `maxLatencyMs` 5000 (in the test only, as I7 would): the scam and
  lookalike cases blocked 5/5 with confidences 0.90–0.99, the benign cases
  allowed 4/5 each (one timeout each), inconclusive (interval 0.38–0.96).
  Kept as recordings, replayed deterministically. The first runs found two
  bugs, fixed before these: the model copied the output's schema (every
  answer invalid) and samples were cut at 10 seconds.
- **I4, a worker on a real model**: in the sample with its code, with the
  link store emptied, the worker implemented `LinkStore` from the model
  twice (25 s and 29 s; one round of checks each; $0.0091 for the second,
  the first not metered), with tests of its own; the Studio's verification
  passed (build, boundaries, 15 contracts, 7 tests, 6 scenarios); after
  integration the six scenarios pass against the code.
- Spend: about $0.08 in all.

**Needs the Operator**

- `LinkScreening`'s `maxLatencyMs = 500` (§2.10) is not met by
  `deepseek-flash` from this machine: raise it, choose a faster model, or
  accept that every link waits for review (ROADMAP §7.6).
- The decisions of 2026-10-01 in §7.6.
- Walking Scenario I (I1–I8) in the Studio, with the Assistant on a real
  provider for I4 and I6 and a live evaluation for I3 (C-15).

**Not done or not tried**

- W6.10: the Assistant modelled as an agent in the self-model (C-44).
- I1 by the Library: the screening agent and its fallback are not a Library
  block of their own (`Moderation` shows the pattern); I1 is walked with the
  sample.
- W8.3's grouped approvals: the Operator starts each task, and reviews each
  patch; the worker's own steps are not approved one by one.
- The model and implementation breaks of the second example run in tests,
  not in a journey.
- The component gallery shows the Surface's run and drift marks, not the
  Run panel or the Scenarios tab.
- Screen readers on the new panels: not tried. Anthropic, OpenAI and
  OpenRouter as the live client or the worker's model: not tried (no keys).
- No evaluation-set tasks for the new tools yet (they would run live).
- Implementation checks are for Rust only (§6.5 "Waits").
- Memory against its budgets: not measured this time.

**Operator: try this**

1. Start from the URL shortener "And its code" (the start screen; with a
   fresh session file it is on the welcome), then Ctrl+Shift+R, choose
   `ReviewRequired`, press F5, step through the trace with `[` and `]`, and
   play it with `\`.
2. In the Inspector, lower `LinkScreening`'s `minConfidence` to 0.5: the
   result turns outdated; run it again: its check fails at the element;
   undo.
3. Choose Code in the Run panel, allow trusted-local execution, and run a
   scenario against the real code; select `api` and "Check the
   implementation".
4. Write a scenario: select the service, "New scenario", then Add: a
   stand-in, what to send, what to wait for, a check; run it.
5. With a key: "Implement with the Assistant…" on a part (empty a file in
   the code folder first to give it work), review the patch, integrate.
   Live: Mode Live, "Evaluate live…" (about a cent).
6. The scripted runs:

   ```text
   cargo build -p agq-studio-native --features automation
   target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-i\session.json --scenario i-scenarios --project %TEMP%\agq-i\UrlShortener --gallery %TEMP%\agq-i\shots
   target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-c\s.json --scenario i-code --project %TEMP%\agq-c\UrlShortener --gallery %TEMP%\agq-c\shots
   ```

   The live runs (they spend money):

   ```text
   $env:AGQ_LIVE=1; cargo test -p agq-studio-native live_ -- --ignored --nocapture
   ```

## Stage 10: Agentique builds Agentique (C-51)

Status: **in progress.** Started on 2026-10-01 under the Operator's direction
(C-51, ROADMAP §6.6), on the branch `stage10/self-hosting`. Nothing here says
the Operator accepted anything; gates A–E are the Operator's (C-53 replaced
gates C and D with Stage 11's autonomous proof, W11.7).

**Baseline** (`main` at `f3d0dae2`, untouched, debug builds with lean
settings): `cargo fmt --all -- --check` clean; `python
tools/check_architecture.py` OK (11 crates in 10 parts, 23 allowed
dependencies); `cargo test --workspace` passes, none failing; `cargo clippy
--workspace --all-targets -- -D warnings` clean.

**Foundation review** (read-only, recorded in ROADMAP §5.6): three workflows
traced through the code (rename a part, the Assistant changing the model, an
implementation task), who owns which state, and the corrections by priority.
Confirmed defect: a task's verification counted a check only when it failed.

**Built** (merged to `main` in #86 on 2026-10-03, which is not acceptance;
decisions pending the Operator are in
ROADMAP §7.6)

| Item | State |
|---|---|
| W10.1 Foundation corrections | Done, each with a regression test written first and seen failing: a check that did not pass counted as a pass; integration of exactly the task commit (not a re-applied patch), leaving the Operator's staged work alone; protected paths whatever their case or spelling; one verdict summary (`agq_simulation::summary`) for runs, tasks and checks; a test found by its whole name only |
| W10.2 The self-model | Done: `model/Agentique.sysml` (12 parts with purposes, owned information and contracts; item and port defs; connections; 5 requirements; 8 workflows with failure paths that run in model execution); 101 links to code and tests; "Develop Agentique" (Welcome and palette); the Inspector's "About this part" and the Assistant's `explain_element` from the same function; `model/README.md` reads it top down |
| W10.3 Runtimes | Built: `Runtime` (the loop, or the Claude Agent runtime); the companion (`claude-agent/`), protocol 1, policy, setup and health in Settings › Assistant; streaming, tool activity, questions, stop, errors, usage and a visible phase in the Conversation; sessions resumed (each turn forks the session it continues) or handed over |
| W10.4 Tasks | Built: required checks fixed at approval, each with an explicit outcome; the coding tools (search, ranged reads, exact edits, allowed programs); the worker on either runtime; the task commit; the worker's model changes in its worktree's model through System State operations, verified against that model, listed by element in the review, checked again and read back at integration; code of locked parts asks at integration. Agentique never pushes |
| W10.5 Builds | Built: a release build of one commit in a detached worktree, with a manifest and the registry; "Try" as a test instance; "Use this build" (digests, commit, data formats, backup, handover to the launcher); the launcher's fallback to the last known good build, `--recover` in safe mode, diagnostics in `launcher.log`; Settings › About › Builds |
| W10.6 Proof | Partly: the tests below; an independent review of the safeguards found eleven defects, all fixed (ROADMAP §7.6); seven have a regression test of their own, while the runtime's stop deadline, the backup, the safeguard links and verification on a clean checkout (which every task test now goes through) do not; the reference budget run not run |

**Checks** (debug builds, lean settings, 2026-10-01): `cargo fmt --all --
--check` clean; `cargo clippy --workspace --all-targets -- -D warnings` and
with `--features automation` clean; `cargo test --workspace` 552 passed, 0
failed, 9 ignored (live or costly runs); `python tools/check_architecture.py`
OK (12 crates in 11 parts, 25 allowed dependencies) and its 14 tests pass; the
companion's 7 tests pass and its type check is clean. Journeys (debug,
`automation`): `a-build`, `a-reopen`, `a-assistant`, `c-understand` (new: the
self-model's "About this part", a dependency and back, the development task's
workflow in model execution), `d-daily`, `e-settings`, `h-library`,
`i-scenarios` and `i-code` pass; `a-crash` exits 3 by design.

**Not tried, or not verified**

- Gate B is **unverified**: no Anthropic key on this machine. The real SDK
  was started with a refused key (no cost) and reported exactly Agentique's
  tools, `dontAsk` and no machine settings; the live test
  (`live_the_sdk_reads_the_model_through_agentique_and_resumes_its_session`)
  waits for a key.
- A release build of Agentique itself was not made: the build pipeline is
  tested on a stand-in workspace with the same package names, and Agentique's
  own build needs this work committed first. "Try this build" and "Use this
  build" were not run end to end; the launcher's fallback is tested with real
  processes.
- The reference budget run (§8.6) was not run: the release build needs more
  memory and disk than this machine had free (3.1 GB of memory, 6.7 GB of
  disk). The Inspector's "About this part" is computed when the Inspector
  renders, like its other sections; not measured on a 10k model.
- CI's new companion step (Node 22 from the runner's tool cache) had not run:
  on `main` CI stopped at checkout (a tracked agent worktree), repaired in
  C-52's step 0a below.
- Gates A–E are the Operator's; none is claimed (C and D are replaced by
  W11.7 under C-53).

## Scenario I's typed decisions, alongside Stage 10 (C-52)

Status: **in progress.** Started on 2026-10-03 under the Operator's direction
(C-52, ROADMAP §6.6): the System One investigation's plan (updated after
PR #86, baseline `219598b6`), built in dependency order as separately
reviewable pull requests. External implementation work: it is not Stage 10's
two-generation proof and claims none of its gates. Paid inference, live
evaluations, production activation and the Operator's acceptance are
separate decisions; nothing here was run against a real provider.

**Reviewable sequence** (each based on the one before):

| Step | Pull request | Branch | What |
|---|---|---|---|
| 0a | #87 | `fix/ci-untrack-worktree` | CI's baseline: the tracked agent worktree, a Linux-only lint, a Linux lock race in a Studio test |
| 0b | #88 | `fix/execution-cross-platform-scope` | Execution reads scoped paths the same way on every host |
| G | #89 | `scenario-i/scope-and-plan` | C-52, the persistence decision named before it is built, the self-model, this record |
| 1 | #90 | `jev/adapter-correctness` | A1–A4 and the validation policy |
| 2 | #91 | `jev/deadlines-cancellation` | The decision handle, one deadline, cancellation |
| 3 | #92 | `simulation/execution-identity` | Call limits, bindings, decision evidence, freshness, legacy records |
| 4 | #93 | `scenario-i/typed-screening` | The opt-in single-choice evaluation in the Studio |
| 5 | #94 | `url-shortener/jev-client` | The URL shortener's real client and conformance tests |
| E | #95 | `scenario-i/evaluation` | The evaluation's definitions and report |
| U | #96 | `providers/rig-0.43` | The rig migration |

**Coverage checklist.** Every implementation item of the investigation's
§3, §5, §6, §7.2 and §8, with its owner, its state and the evidence that
accepts it. States: *done* (built and tested offline), *planned*,
*not started*, *waits* (needs a decision or consent named in the row).

| Item | Owner | State | Acceptance evidence |
|---|---|---|---|
| §8 0a: tracked worktree gitlink removed, worktrees ignored | `.gitignore`, the index | done | fresh clone: checkout's `git submodule foreach` cleanup exits 0 (128 on `main`); CI reaches its checks |
| §8 0a: Linux lint (`unused_mut` in `claude_agent.rs`) | `crates/assistant/src/claude_agent.rs` | done | `cargo clippy --workspace --all-targets -D warnings` clean on Linux (WSL Ubuntu 24.04) |
| §8 0a: Linux lock race reopening a project | `crates/studio-native/src/studio.rs` | done | the Studio's unit tests on Linux: 1 of 6 full runs failed before, 14 of 14 passed after |
| §8 0b: host-independent scoped paths | `crates/execution/src/lib.rs` | done | historical assertion reproduced failing on Linux at `219598b6`, passing after; Windows and Unix forms and a link (junction on Windows) tested on both hosts |
| §3.1 A1: complete bounded success bodies, bounded sanitized error excerpts | `providers/src/jev.rs` | done (1) | the >300-character fake-server regression failed first ("EOF … column 300"), then passed; chunked with a split two-byte character; announced and streamed oversize refused; cut-off read; 20 KB error reply excerpted without the key |
| §3.1 A2 and §3.2: request-bound validation (IDs, kinds, model, options, legend, distributions, selection) | `providers/src/jev.rs` | done (1) | `read_reply(request, …)`: inline matrix (envelope, noul, choice, score) and HTTP cases (wrong model, unknown option, malformed success not retried) |
| §3.1 A3: score keys canonical and exact | `providers/src/jev.rs` | done (1) | `x`, `-1`, `01`, `00`, `+1`, missing `0`, extra `3`, duplicate keys and keys colliding once unescaped all fail |
| §3.1 A4: usage known, partial or unknown, never a false zero | `providers/src/jev.rs`; callers and summaries | done (1, 3, 4) | omitted, null, partial, reported zero and invalid counts distinguishable; failures count requests sent and keep reported usage; the example logs unknown cost at the worst case |
| §3.2 numeric tolerances with boundary fixtures, raw values kept | `providers/src/jev.rs` | done (1) | sums at ±0.01/±0.02 (hundredths) and ±1e-3; ties within 1e-6; score mean within 0.016 and 0.005; boundaries 0, 1, −0.0; NaN, infinity and 1e400 refused as JSON |
| §3.1 A5: no silent chat for a decision model; capability-based resolution | `providers/src/capabilities.rs`, `studio-native/src/live.rs` | done (4) | `resolve_model` and `Capabilities::{chat, decisions}` (no prefix match; known pins only); Studio: the typed agent asks only decisions; `jev-latest` and an unknown id refused, never replaced; a missing TypeSafe AI key blocks the plan and another provider's key is not borrowed |
| §3.1 A6, §5.4: one monotonic deadline at the provider boundary | `providers` (2), `simulation` (3), `studio-native` (4) | done (2, 3, 4) | `decide_start(request, deadline)`: a silent server and a stalled body end at the deadline (within 250 ms); a late reply is a timeout; a retry is made only when its wait fits |
| §3.1 A7, §5.5: execution identity, freshness, legacy records | `simulation`, `studio-native` | done (3) | `simulation/tests/identity.rs`: legacy canonical bytes unchanged; every binding part changes the key, object key order does not; legacy recordings replay unbound requests and never bound ones (the stop says why); a damaged key is not used; freshness by binding, runner and recordings; an older reader's types parse new lines and never match them. Studio: replay outdated when the Assistant's model (left to it by the agent) changes, and its recordings no longer answer; a legacy replay result is outdated |
| §3.1 A8: honest estimates, bounded calls and attempts, known live cost in the daily total | `studio-native` | done (4) | the plan's allowance (samples × calls, 10 per sample without a model run) and requests per call; a cost upper bound from request bytes at the dated list price; unknown cost said in the Run panel; known live cost added to the day's total |
| §3.3: abort on timeout, shared client, credential precedence, replay fails closed, provider failure vs semantic failure | `providers`, `simulation` | done (1–3): preserved, with tests | existing tests kept; connection reuse and cancel-before-error tests added |
| §3.4: documentation corrections (README, `live_model` comment, retry policy, capability text, prefix prices, fixture comments) | READMEs, `live.rs`, `capabilities.rs`, the fixture's `screening.rs`, `worker.rs` | done (1, 4, 5) | prices of known pins only; the stale `live_model` comment replaced; the fixture says a real client exists; the worker's module comment says it changes its worktree's model through System State operations |
| §5.2: admission (one effective configuration, known pin, flat enum 2–255, confidence by identity) | `studio-native/src/live.rs`, read-only agent description in `simulation` | done (3, 4) | two configurations, an alias, an unknown id, a second required field and an undocumented value each refused with the reason, before consent |
| §5.2: state preparation and deterministic question mapping | `studio-native/src/live.rs`; enum value docs in the model | done (4) | `decision_request`: exactly `longUrl` and `host` under the item's name with a note that they are data; missing, nested or mistyped fields refused, not repaired; the same binding for replay |
| §5.3: condition table (valid, review, low confidence, deadline, invalid, provider failure, cancelled, allowance used up) | `simulation`, `studio-native` | done (3, 4) | fake decision service: confident screening passes all four cases; unsure answers go to the fallback (`lowConfidence`); 700 ms answers time out at 500 ms (20 timeouts, no retry); 401, an unknown option and another model version end the evaluation as `providerError` with no verdict |
| §5.3: fallback is not a mandatory pre-call blocklist | `models/link-screening`, docs | done (4) | said in the plan the Operator confirms |
| §5.6: frozen consented plan, invalidated on change; effective model, data sent, attempts, estimate, unknown usage, confidence meaning, fallback and error reasons, stale results | `runs.rs`, `dialogs.rs`, `panels/run.rs` | done (4) | a change after confirmation refuses the start with nothing sent; a change during the run stops it (cancelled, outdated, nothing sent after); the dialog shows `LivePlan::lines`; the Run panel `live_lines`; the Inspector's Agent section says how a live evaluation asks the model (`model_call`); journeys `i-scenarios` and `i-code` pass |
| §5.7, §8 5: real Rust client; same six scenarios; frozen-response conformance with the held-link regression | URL shortener fixture (`src/jev.rs`, `tests/jev_client.rs`, `tests/decisions.json`), `implementation/tests/url_shortener.rs`, the Studio's sample | done (5); production adoption waits for the benefit gate | `JevClient` (pinned model, the model's question and documented options, the two declared fields as data, the reply checked as in Providers, `maxLatencyMs` as the deadline, retries only when they fit; failures go to the fallback, never allow); the harness still answers with stand-ins and the six scenarios pass against the code; eight frozen replies give the same decision and decider in the code and (four of them) in the model's typed evaluation; the deliberate `review → active` break fails the linked conformance test, shown as drift at `TypedLinkScreening`; a test checks the code's question, options and pin against the model's text. No TLS in the dependency-free fixture: `PlainHttp` serves local endpoints and a deployment supplies a TLS transport |
| §6.1: self-model before cross-part changes | `model/Agentique.sysml` | done (G) | architecture check and dogfood test green |
| §6.2: interface sketches as built (handle, `read_reply(request, …)`, `CallLimits`, binding, Choice mapping, both `from_json` consumers) | as above | done (1–5) | the Studio's flat `{decision, confidence}` is read by `from_json` in every live test; the harness's `{type, fields}` envelope is unchanged and its six scenarios pass |
| §6.3: no library extraction | — | done (decision) | two consumers share fixtures, not a crate |
| §7.2 body handling | `providers/tests/jev.rs` | done (1) | fake HTTP server (see A1) |
| §7.2 schema and numerics | `providers/src/jev.rs` tests | done (1) | inline and wire (see A2, A3, §3.2) |
| §7.2 status and retry | `providers/tests/jev.rs` | done (1, 2) | 400/401/402/403/404/413/422/500/418 never answers and never retried; 529 three times stops at three requests; an 11 s wait not waited for; a 20 ms wait retried |
| §7.2 cancellation and deadline | `providers`, `studio-native` | done (2, 4) | silent server, stalled body, late reply, retry that does not fit, stop during a retry wait (result within 100 ms, no request in the next 1.8 s), drop closes the connection, a stop wins over a reply or an error already on its way, a past deadline sends nothing, connections reused across three decisions |
| §7.2 credentials and privacy | `providers` | done (1) | dummy keys only: no request without TypeSafe AI's own key (missing, blank, another provider's); a child process with an ambient key shows an endpoint override never gets it and an explicit key wins; no key in `Debug` or in error excerpts |
| §7.2 routing and consent | `studio-native` | done (4) | see A5, §5.2, §5.6 |
| §7.2 replay and freshness | `simulation`, `studio-native` | done (3, 4) | see A7; replay never reaches a live client (no `LiveModel` exists in replay) |
| §7.2 scenario and code safety | model and fixture tests | done (4, 5) | six scenarios against the model and the code; only an active link redirects in every frozen case; timeout, unreachable, too large, 401 and 500 never allow (a blocklisted host is blocked); a confident wrong allow is shown to activate, as the fallback is no pre-call check |
| §7.2 authority boundaries | existing worker, task and Execution tests | done (4) | after a typed evaluation of confident answers the System State revision is unchanged and no task exists; the live client has no tools; the existing worker, task and Execution tests still pass |
| §7.3–§7.6: evaluation definitions and reporting (quality, calibration, latency, coverage, economics); proposed thresholds | `screening_evaluation` (ignored test in `studio-native/src/live.rs`), `tools/screening_report.py` | done as definitions; nothing paid was run | the runner takes the Operator's labelled cases (or the model's four smoke cases) through the typed, chat and blocklist arms with the engine's own contract, needs consent, an allowance and the spend stop, and writes observations outside the repository; the report gives confusion, exact bounds, Brier and log loss on a normalised copy, reliability bins, p50/p95 with timeouts at the deadline, coverage, unknown cost and cost per correct workflow, domain-clustered paired bootstraps, and the §7.7 gates marked PROPOSED; its arithmetic is tested on synthetic observations; the free arm ran offline end to end |
| §7.7: go/no-go and rollback | this record, ROADMAP, `tools/screening_report.py` | done as far as code goes; the decisions are the Operator's | the gates are printed as PROPOSED; rollback paths: an unbound or legacy run is never current (3), an older build reads new data safely (3), a typed agent whose model is not known is refused, never answered by another model (4), the rig change is its own pull request (U) |
| §8 U: rig 0.43 migration, provider parity, Windows TLS, dependency review | `crates/providers` (`chat.rs`, `fallback.rs`, capabilities), `Cargo.lock`, the evaluation example | done offline (U); the live five-task evaluation on every Assistant provider waits for the Operator's consent and keys | rig-core =0.43.0 (native-tls, no default features); the canned-provider suite (Anthropic, OpenAI Responses, OpenRouter, DeepSeek: text, reasoning, tools, retries, refused keys, cut streams, cancellation, fallbacks, unreadable tool input) and the workspace pass on Windows (MSVC, schannel) and Linux; one rig-core line; added rig-http, rig-reqwest (and rig-tungstenite in the lock, not built), removed as-any, eventsource-stream, tracing-futures; no new duplicate, no rustls, ring or aws-lc in Providers' tree; all MIT. rig-typesafeai evaluated and not adopted (see the decision log). Not tried: a live TLS handshake with each vendor |

**Deviations** (one line each, with the reason):

- The investigation's "execution descriptor" is called a **binding** in code
  (`AgentRequest.binding`, `Provenance.binding`): it says how an agent's call
  is bound to a provider, and "execution" already names a part.
- 0a also carries two Linux-only CI fixes found once checkout worked again (a
  lint and a test's lock race); they were hidden behind the checkout failure.
- Thresholds and deadlines are not in a binding (the investigation listed
  "policy" with them): they are applied to a recorded answer on replay, and
  the model digest already outdates results when they change; keying on them
  would force paid re-recording without changing what the model was asked.
- A provider failure ends a live evaluation (no later sample sends); before,
  each later sample asked again.

**Compatibility and rollback (step 3).** New data is read by the previous
build (`main` at `219598b6`) as follows, from a run of that build's reader on
data this build wrote (Linux, outside the repository): bound recordings are
read and never matched (0 of 4); a result stopped as `budget-exhausted` is
skipped (2 of 3 listed); a live cost with unknown parts reads as unknown. It
shows a bound live or replay result as current while the model is unchanged,
as it always did for live results. Recordings made before this build replay
unbound requests only; in the Studio every replay is bound, so they must be
recorded again (the stop says so). Results from before are outdated, never
rewritten.

**Checks** (2026-10-03; Windows 10 with Rust 1.97.1 MSVC, debug builds with
lean settings; Linux is WSL Ubuntu 24.04 with Rust 1.97.1 and Node 22,
running CI's steps on a clean checkout of each step's commit): at E, on
Windows, `cargo fmt --all -- --check` clean, `python
tools/check_architecture.py` OK (12 crates in 11 parts, 25 allowed
dependencies), the tools' 21 tests pass, `cargo clippy --workspace
--all-targets -D warnings` and with `--features automation` clean, `cargo
test --workspace` 606 passed, 0 failed, 11 ignored (live or costly runs, and
the evaluation runner). On Linux every step from 0b on passes CI's steps
(tests 561 at 0b to 605 at 5); GitHub's CI ran on #87–#95: #88–#95 pass,
budgets included; #87 alone fails only at the Linux path assertion #88
fixes. At U on Linux: every CI step passes, 616 tests, the three CPU
budgets. Journeys (debug, `automation`, at U): `i-scenarios` (53 steps, the
live confirmation opens and Escape sends nothing), `i-code` (14),
`c-understand` (9), `a-build` (74) then `a-crash` (exits 3 by design) then
`a-reopen` (15) pass. Reference run (§8.6; release, `automation`, at U, this
machine): warm start to first update 248 ms (budget 400); 1k pan and zoom
frame interval p95 6.2 and 6.2 ms (8.3), input to next update 0.6 and 0.7 ms;
10k 12.8 and 12.8 ms (16.7), input 0.6 and 0.7 ms; chat scroll 6.2 ms and
streaming 6.5 ms p95. Every budget is met.

**Not tried, or not verified**

- No paid call and no live evaluation: no key was used. The typed
  evaluation, the chat baseline and the rig upgrade's five tasks on every
  Assistant provider wait for the Operator's consent, keys and spend stop.
- The 30-minute soak and memory were not measured; the journeys ran on a
  debug build; screen readers were not tried.
- A live TLS handshake with each vendor after the rig upgrade (local
  servers are plain HTTP).
- Production use of the URL shortener's client: its transport has no TLS
  (a dependency-free fixture), and adoption waits for the benefit gate.

**For the Operator to decide**: the persistence change as built (§7.6);
the C-34 exception for Jev (keep the thin client); the rig upgrade's
changed behaviour (tool input no longer streamed; unreadable tool input
outside Anthropic ends the reply); the screening risk and coverage
thresholds (printed as PROPOSED); whether URL-only evidence suffices; the
live evaluations' data, providers, attempts and spend; acceptance of
Scenario I after use (C-15).

## Stage 11: Agentique improves itself (C-53)

Status: **in progress.** Started on 2026-10-03 under the Operator's direction
(C-53, ROADMAP §6.8): autonomous, self-improving development through
Agentique. Nothing here says the Operator accepted anything.

**Baseline** (`main` at `f7a891da`): Stage 10's runtime, tasks, builds and
launcher, and C-52's typed decisions, as recorded above.

**Work items**

| Item | Pull request | State |
|---|---|---|
| W11.1 Direction and self-model | #97 | C-53 recorded; Scenario J, §4.16, §6.8; `AGENTS.md` rule 10; `CLAUDE.md`; the self-model gains `Orchestrator` (locked, through a System State `Lock` change), the control and objective ports, `GatesDecide`, and the new contracts of `ClaudeAgentRuntime`, `Studio` and `Launcher` |
| W11.2 The development runtime | `stage11/dev-runtime` | Built: protocol 2, the permission policy (companion hook and Rust policy), development sessions in the Conversation for projects with implementation links, DeepSeek's endpoint, steering, activity, costs; live development session passed on `deepseek-v4-pro` |
| W11.3 The control interface | `stage11/control` | Built: the per-frame control registry (regions for the cached columns and the views inside them; controls in painting order, clipped to what is visible), observations, actions through real input and command dispatch, stale refusal (instance and the observed screen revision always; project, build and session when given; the selection is part of the screen), the Operator's own refused to agents where each effect happens (approval dialogs and the Operator's questions, Settings, the Conversation, locking, undo, appearance, the agents chip), whatever route reaches it, with an agent's changes recorded as the Assistant's, waits beside actions, expired requests dropped, the event trace, marks and the agents chip with Pause, Step, Resume, the local endpoint (`--control`; bounded lines and connections, an OS-random token), `observe_app` and `act_in_app`; the journey `control_journey` drives the real window through the endpoint, including the refusals |
| W11.4 Lifecycle | `stage11/control` | Built: the supervising launcher (handover by exit code 75 and `handover.json`, which names a registered build, the session and the project; an exit without a valid handover counts as a crash; crash restart once, then the build that was last known good before it started; a crash an hour after starting counts as a first one), `--supervised` and `--adopted` in the Studio (also on the one-shot `--adopt` path), the check after adoption before the ready file, the Studio's work stopped before it hands over; continuation of objectives comes with W11.5 |
| W11.5 The Orchestrator | `stage11/orchestrator` | Built: the crate `agq-orchestrator` (record and keyed journal, the cycle's phases, gates for protected paths, agent configuration at any depth, Agentique's safeguards, keys in the change and in everything pushed, locked elements, the code of locked parts through the links, criteria that fail before the change and run tests after, and a line-level baseline guard; roles as separate Claude Agent sessions with one hand-over tool each, the lead and the reviewer running no commands, worktree sessions refused the commands that move the repository's shared state; one squashed commit of the reviewed tree pushed per review, an idempotent merge, release and debug builds, test instances without the keys of the Studio's environment driven through the control interface), budgets and no-progress stops, Pause, Step, Resume, Stop and messages that reach a working agent, interruption that leaves an objective to continue, and continuation in an adopted build (a build that does not take over ends the objective); the Studio's Objectives panel and its status line; agents may not operate the panel. Tested: unit tests, and `tests/cycle.rs` runs cycles end to end with a scripted companion (a proposal whose criterion fails on the base, a key-gate failure, a repair, checks on a clean checkout, independent review, a merge the permissions refuse; and an interruption continued to the end). Live use comes with W11.7 |
| W11.6 Typed decisions in operation | `stage11/decisions` | Built: `decide` (a situation read from an observation, with the option already selected; the forward and the cancelling rules; Jev's typed choice under a 4 s deadline with a 0.6 confidence threshold; escalation to the reasoning model on low confidence, an invalid answer, a timeout or a failure; an approval, or a dialog with nothing to press, waits by rule; a failed call's time and cost are counted); before each observation criterion in Evaluate and Try, the Orchestrator cancels a dialog left in a test instance's way by rule (only Cancel is pressed; the dialog is named in the outcome; when the way cannot be cleared, the criterion is not run, which is no pass), a build that starts with a dialog open fails, a setup action that fails fails its criterion in Try as in Evaluate, and the lead is told each observation criterion starts with no dialog open. The cancelling rule is tested on a real Studio, guarded (ten dialogs cancelled, the goal reached, nothing changed, no model asked); a Jev-assisted application-control workflow is measured live against the rules and the reasoning model (below); the control interface refuses the system's folder picker to agents |
| W11.7 Proof | #103 (bootstrap); #104, #105, #106 by Agentique | Run live (below), with an AI agent standing in for the Operator: after the bootstrap, Agentique merged and adopted three improvements of itself from objectives entered in its window (a correctness fix in edge routing, then two comprehension fixes, each started in the build the previous one adopted or recovered to), with interruption and resume, a failing gate repaired, a judgment no test instance could meet keeping a cycle from merging until it was stopped, a stale action refused, and a failed launch recovered twice. Not done on the final `main`: the Windows journeys and the reference run. Waits for the Operator's own run and acceptance |

**Capability checklist: the Claude Agent runtime against the Claude Code
CLI** (W11.2). "Before" is `main` at `f7a891da`; "Now" is W11.2 as built:
*live* means seen working against DeepSeek's endpoint through Agentique's
runtime (`live_a_development_session_on_deepseek_works_in_the_repository_within_its_policy`,
2026-10-03, 25 s); *tested* means the companion's or the Rust tests with a
stand-in; *configured* means passed to the SDK and reported by it, not
exercised by a test.

| Capability (Claude Code CLI) | Before | Now |
|---|---|---|
| Coding and reasoning model | Anthropic models with an Anthropic key only | Anthropic, or DeepSeek's Anthropic-compatible endpoint (`deepseek-v4-pro`, `deepseek-flash` for small tasks) with the DeepSeek key: *live* on `deepseek-v4-pro`; Anthropic with an API key not tried (no key); since W12.3 Anthropic also on the Operator's Claude subscription token: *live* on `claude-sonnet-5-5` (2026-10-04) |
| File search, read, edit, write | Off | The SDK's own tools in the repository, held to the policy's folders, judged on the real path (links, junctions, short names) and refusing stream or trailing-dot names; model files, agent configuration, `.git` and the links' protected paths refused with the reason; key files hidden at any depth (also as the SDK's own Read deny rules for its search): *live* (edit made, model-file edit refused naming `apply_changes`) |
| Commands (Bash, PowerShell) | Off | On with trusted-local execution; standard refusals (force-push, push to `main`, merging, repository changes on GitHub, global git configuration, key files, writing model files; in the Operator's working copy, discarding work); not confined to a folder: *live* (a `node` command), refusals *tested* in Rust and the companion |
| Builds and tests | Only through a task worker's allowed programs | Through commands as above (Cargo, Python, Node with the Operator's toolchains and home folder): *configured*; exercised in W11.5's cycles |
| Git and GitHub | Integration by the Studio only | Reading git and local commits; pushing refused in the Conversation; merging refused for agents; pushes and pull requests are the Orchestrator's (W11.5): *tested* |
| Subagents | Off | The SDK's own (general-purpose, Explore, Plan) and the project's; their start and end shown in the Conversation: *tested* (stand-in), *configured* |
| Custom tools | Agentique's MCP tools | Agentique's tools beside the SDK's, through the Studio's executor: *live* (`read_model`) |
| MCP servers | Agentique's only | Agentique's, plus servers a policy names (none by default): *tested* |
| Project instructions | Off | `CLAUDE.md` importing `AGENTS.md`, loaded (`settingSources: ["project"]`): *configured*; seen live in the 2026-10-03 spike |
| Skills | Off | The project's skills (`skills: "all"` with project settings): *configured*; this repository has none yet |
| Hooks | One hook that denies everything else | The policy's hook (it re-checks Stop after a pause, and holds a paused call for up to a day); the project's own hooks run only with trusted-local execution, and the agent configuration (`.claude/`, `.mcp.json`, `CLAUDE.md`, `AGENTS.md`) is protected unless an objective names it; the endpoint, the key scrub and proxies are pinned above the project's settings: *tested*, and *live* with commands off (the policy hook still refused a model-file edit and a command) |
| Persistent context | Off | Project instructions; auto memory stays off (C-40): *configured* |
| Compaction | The SDK's, not shown | The SDK's, as a notice with the token counts: *tested* |
| Resumable sessions | Each turn forks the session it continues | The same: *live* (the second turn resumed and answered from memory) |
| Long-running work | 40 model calls a turn, no background commands | Background subagents and commands keep the turn open until they end (at most 30 minutes), and their start and end are reported; queued messages may fold into the running turn (the SDK's own count of pending sends decides when it ends); the Conversation's bound stays 40 calls a turn (objectives set their own, W11.5): *tested* |
| Permissions | `dontAsk`, Agentique's tools only | The policy's hook allows, refuses, or asks; a question goes to the Operator as a card with Allow / Don't allow and is recorded; objectives refuse instead of asking: *tested* |
| Web fetch and search | Off | Fetch when the project allows the network; search only where the endpoint offers it (Anthropic; DeepSeek ignores server tools): *configured* |
| Steering | Stop only | Queued messages ("Add" while it works), Pause before the next tool call, Step, Resume, Stop: *tested* (stand-in and companion) |
| Keys | The Anthropic key in the companion's environment | The model's key only, kept out of the session's commands, hooks and MCP servers (`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`, measured with and without on 2026-10-03). Since W12.3: exactly one credential (an API key, or the Claude subscription token with no key beside it), checked against what the SDK reports before the first prompt reaches the model, and a session that would use another (such as this computer's claude.ai login) stops: *tested* (stand-ins), *live* on DeepSeek's key and on the token, whose session's command saw neither the token nor a key (2026-10-04) |
| Cost | The SDK's estimate | From the usage of every model a turn used (subagents, compaction and small tasks included), each result's own share, each model at its own dated price (an unknown model at the turn's): *tested* |


**W11.7 measured** (live, 2026-10-04, on the reference machine; the Claude Agent runtime through DeepSeek's Anthropic-compatible endpoint, `deepseek-v4-pro`; times in UTC)

Who did what. No person was at the machine. An AI agent (Claude Code, working on this repository) stood in for the Operator: it bootstrapped, wrote the four intents, decided when to pause, step, resume, close, continue, stop and restate, and gave that input as real mouse and keyboard input in Agentique's window, at the positions the control interface reported. Agentique's own agents (lead, implementer, reviewer, evaluator) and the Orchestrator did the rest. Two demonstrations used scripts outside Agentique, named below: the injected crash and the stale action. Nothing outside Agentique edited or repaired the repository or the app after the bootstrap. The Operator has not yet entered an objective or watched one, which the W11.7 gate requires (§6.8).

Bootstrap, once: a Studio built from `main` (`9c24dc99`) opened the repository; trusted-local execution was allowed; Settings › About › Builds built `main`, and "Use this build…" restarted Agentique in it, which installed the launcher; a desktop shortcut starts `agentique-launcher --supervise --project <repository> --control <file>`. One defect found while preparing it was fixed before the proof started (#103: an installed build did not know its repository).

| Objective (intent entered, new build ready or stopped) | From build | Result | Attempts | Agents' time | Spend | Pull request |
|---|---|---|---|---|---|---|
| 1, correctness: collapsing a container drew the connections inside it as loops on its card (00:52:47–01:21:38) | `9c24dc99` (bootstrap) | Done, adopted | 1 | 28 min | $0.74, 3.0 M tokens | #104, merged as `1ef86e81` |
| 2, comprehension: "Check the implementation" hid why `model/links.json` could not be read (01:22:47–01:38:07) | `1ef86e81` (adopted from 1) | Done, adopted | 1 | 15 min | $0.42, 2.6 M tokens | #105, merged as `86b79610` |
| 3, comprehension: after a failed launch the recovered Studio's status line said only "Opened …" (01:43:39–02:42:41) | `86b79610` (recovered) | Stopped, nothing merged | 2 | 59 min | $3.35, 20.9 M tokens | — |
| 3 restated: the same, to be checked with a test (02:43:16–03:05:50) | `86b79610` (recovered) | Done, adopted | 2 | 23 min | $0.73, 4.4 M tokens | #106, merged as `ce5408a8` |

Each finished objective went through the whole loop without help:

1. The lead's proposal with frozen criteria; each criterion was a new test, which ran no test on the base.
2. The implementer, in its own worktree.
3. The required checks, the criteria and the gates, on a clean checkout.
4. The independent reviewer's approval.
5. One squashed commit of the reviewed tree pushed, a pull request opened, CI ("workspace") passing on that commit, and a merge pinned to it.
6. A release build of the merged `main`, then a trial in a test instance.
7. The handover to the supervising launcher (exit 75), which started the new build `--adopted`.
8. The objective resumed and closed in the new build.

The second objective was started in the build the first adopted; the third and its restatement were started in the build the launcher recovered to.

Shown on purpose:

- **Interruption and resume.** In objective 1, Pause held the implementer before its next tool calls, Step let one through, and Resume went on. Agentique was then closed while the implementer was working, and the launcher ended with it. After a start from the shortcut, the panel said "interrupted · $0.40 of $5.00" with Continue. Continue went on in the same worktree, whose uncommitted work was intact, and the objective finished.
- **A failing check.** In objective 3 restated, the first attempt passed the checks and its criterion but failed the gate "code of locked parts unchanged": the change edited `develop.rs`, which holds `Studio::use_build`, a symbol linked to the locked Launcher part. The cycle went to Repair, the implementer moved the change to `studio.rs` only, and the second attempt passed. In objective 3, every deterministic check passed but a judgment criterion could not be met: test instances are never started with `--recovered-from`, so there was no recovery to observe. The evaluator reported it "not run" in the first attempt and "failed" in the second; neither is a pass, so nothing was pushed. The stand-in Operator then stopped the objective, saving at most one attempt (the rule "no fewer failures in two rounds" would have ended it after a third), and restated the intent.
- **A stale action.** A script outside Agentique started a test instance of the adopted build and drove it over two control connections. After the second connection opened Settings, the first connection's click based on its earlier observation was refused ("stale: the screen changed since you observed it (now: settings); observe again"), as was an action expecting another Studio instance. After observing again, its next action (Escape) was carried out.
- **A failed launch with recovery.** Twice, a fresh build of `main` was chosen with "Use this build…", and a script outside Agentique killed its process 0.1 s after it started (an injected crash). The supervising launcher recorded "did not start: it exited (exit code: 0xffffffff) before it was ready", marked the build failed, and started the last known good build with `--recovered-from`; Settings › About › Builds said why. The first time, the recovered Studio's status line said only "Opened …", the defect objective 3 restated fixed. After #106 was adopted, it said which build did not start, why, and that this is the last known good version.

Found during the proof and not fixed (each could be an objective):

- The command palette's rows are not controls in an observation: an agent can open the palette but cannot see what it lists (commands are run by id).
- The Objectives panel's merge and adopt switches have no visible labels.
- Settings › Assistant's runtime card says "Anthropic only" and asks for an Anthropic key, although DeepSeek's endpoint works and was used throughout.
- `act_in_app` passes `observed` to the control interface as the model sent it; a model that sent it as text was refused ("must be integer").
- A judgment criterion that no test instance can reproduce cannot pass; verdicts worded differently each time escape the same-failures rule, and only "no fewer failures in two rounds" ends such a cycle, after three attempts.
- The locked-code gate works at file level: a symbol link of a locked part protects its whole file.
- After a recovery, the recovery message stays in front of every later "Opened …" in that session.
- The Orchestrator keeps each cycle's worktrees (base, verify, trial, work) registered in the repository; 15 after this run.
- Clicks sent by input injection to the title bar's buttons (Settings) did nothing, though the keyboard shortcut worked; whether a physical mouse behaves the same was not checked.

Checks after the proof:

- **Repository checks:** the final `main` (`ce5408a8`) has exactly the tree the Orchestrator checked for #106 on a clean checkout, and all passed there: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`, `python tools/check_architecture.py` and the companion's tests. CI passed on it, as on each merge before it.
- **Reference budget run,** on `main` before the proof (`9c24dc99`): all budgets passed.
  - 10k: pan p95 16.3 ms (budget 16.7), zoom p95 14.1 ms.
  - 1k: pan p95 6.3 ms (budget 8.3).
  - Warm start: 255–285 ms (budget 400).
  - Peak private memory: 156 MB at 1k, 356 MB at 10k, 105 MB on the start screen.
- **Not done on the final `main`:**
  - a repeat of the local checks (stopped when the machine ran low on memory);
  - the ten scripted Windows journeys;
  - the reference run.

  The proof's changes since `9c24dc99` are a filter in the Surface's edge routing (#104) and two status messages (#105, #106).

The proof waits for the Operator's own run and acceptance (§8.3).

**W11.6 measured** (live, 2026-10-04, on the final code; Jev `jev-1.13.0` through TypeSafe AI, the reasoning model `deepseek-v4-pro` through DeepSeek; results kept outside the repository)

Decisions on 33 situations from the Studio's real dialogs (`crates/orchestrator/tests/fixtures/decisions.json`: the goal, the dialog's controls with the option already selected, and the controls that make progress without loss; four are approval dialogs; three tasks accept two next steps, naming the element or choosing its kind; a harmful error confirms what should not be, or answers for the Operator). Latency and cost include failed calls:

| Way | Right | Harmful | Failed | Latency p50 / p95 | Cost (33) |
|---|---|---|---|---|---|
| Forward rule (an empty field, else confirm; approvals wait) | 21 | 8 | 0 | 0 / 0 ms | $0 |
| Cancelling rule (Cancel; approvals wait) | 11 | 0 | 0 | 0 / 0 ms | $0 |
| Jev alone | 28 | 0 | 0 | 230 / 312 ms | $0.0009 |
| Reasoning model alone | 33 | 0 | 0 | 2.6 / 4.3 s | $0.025 |
| Jev, escalating to the model (5 escalated) | 29 | 0 | 0 | 0.24 / 3.3 s | $0.0048 |

Jev alone answers about eleven times faster than the model at the median, at about a twenty-seventh of its cost. Its five errors all fill a field where the goal needed the dialog cancelled or confirmed; four were made with confidence 0.61 to 0.89, which escalation cannot catch. Escalating below 0.6 (five tasks) recovers the fifth, at about a fifth of the model's cost. Half an hour earlier, the same tasks gave Jev 28, the model 33 for $0.061 (p95 7.1 s), and escalation 30 for $0.0056: the model's cost and tail vary between runs. The labels changed before these runs, so earlier runs are not comparable: the dialog now says which option is already selected (pressing it is no progress), the system's folder picker is left out (agents cannot operate it), and only the three naming-or-kind tasks accept a second answer (an earlier version accepted one for seven).

The workflow (`crates/orchestrator/tests/workflow.rs`): a test instance of the Studio, driven only through its control interface; ten dialogs the Studio really opens stand, one at a time, in the way of a goal ("Show the graph view of the model"); each way decides what to press, unguarded, the decision is carried out, and the step is taken. Success is the goal reached with the model and the project unchanged:

| Way | Succeeded | Harmful | Blocked | Latency p50 / p95 | Cost |
|---|---|---|---|---|---|
| Forward rule | 0 of 9 | 3 | 6 | 0 / 0 ms | $0 |
| Cancelling rule | 10 of 10 | 0 | 0 | 0 / 0 ms | $0 |
| Jev alone | 5 of 10 | 1 | 4 | 233 / 331 ms | $0.0003 |
| Reasoning model alone | 9 of 10 | 1 | 0 | 2.8 / 13.1 s | $0.016 |
| Jev, escalating | 8 of 10 | 1 | 1 | 3.7 / 25.6 s | $0.029 |

(The forward rule's run had nine: one dialog did not open after an earlier confirmation changed the model.) Every way that asks a model made one harmful choice: the model confirmed the Checkpoint dialog, and Jev and the escalating way confirmed Save to library. Jev is unsure about most dialogs unrelated to the goal, so the escalating way paid for both and here cost more than the model alone; an earlier run gave the model 8 of 10 for $0.069 (p95 92 s) and escalation 8 of 10 for $0.021. For a dialog unrelated to the goal the answer is known, and the cancelling rule beats every model at no cost; so the Orchestrator cancels by rule (§4.16), and typed decisions are left for questions no rule answers, where the first table shows Jev escalating keeps most of the model's accuracy at a fraction of its cost. No judgment passes a criterion either way.

## Stage 12: Agentique tests and improves itself (C-54)

Status: **in progress** (its mechanisms merged; its proof, W12.7, runs as Stage 13's W13.7 under C-55). Started on 2026-10-04 under the Operator's direction
(C-54, ROADMAP §6.9): from one intent, Agentique explores its own running
application, reproduces what it finds, fixes and adopts, and explores the
adopted version with what it learned. Nothing here says the Operator accepted
anything.

**Baseline** (`main` at `388b5dba`; CI green; the adopted build `ce5408a8`).
Read against the code before any change (file:line on that commit):

- One provider and model for every role: `objective_setup` builds one runtime
  factory that ignores the role (`studio-native/src/objectives.rs:94-133`);
  on DeepSeek every role is `deepseek-v4-pro`, effort unset; the Settings
  model choice is used only by the Conversation.
- `act_in_app` is refused to every model: its `observed` field is declared
  `integer`, which the tool-input checker does not know
  (`assistant/src/tools.rs:534-541`), so "`input.observed` must be integer"
  answers integers and text alike. The endpoint path skips the checker, which
  is why the journeys and the Orchestrator's own client worked; in the W11.7
  proof the evaluator agent could not act (its "a model that sent it as text"
  diagnosis was incomplete).
- Title-bar buttons ignore operating-system clicks (physical or injected): the
  bar is one `WindowControlArea::Drag` with the buttons inside it, and GPUI
  answers `WM_NCHITTEST` with the first window-control hitbox under the
  pointer, the bar's, so the press becomes a window move
  (`workspace.rs:1021-1022`; gpui-pre 0.3.7 `window.rs:1947-1957`). The
  control interface's clicks bypass `WM_NCHITTEST`, so they worked.
- Not in observations: the palette's rows and search field, every switch
  (the Objectives panel's merge and adopt switches, Settings' toggles), the
  Inspector's Name, Multiplicity and Docs fields, the outline filter,
  Settings' text and key fields, the title bar's search and window controls.
  No test asserts that controls have readable labels.
- Typing by agents is applied in one frame; there is no speed setting; the
  event trace is in memory only.
- Agents are refused the Objectives panel (`control/mod.rs:537-539`) and the
  Conversation (`control/mod.rs:476`); no agent can delegate.
- Command-only objectives skip evaluation in a test instance
  (`orchestrator/src/run.rs:1378-1386`); nothing requires a behavioural
  criterion for a user-facing change.
- "Fails before the change" accepts any outcome but "passed" on the base:
  zero tests, a compile error and "not run" all count
  (`run.rs:1332-1355`, `1110-1165`).
- Failure identity is the text of failing lines with digits removed; a
  judgment worded differently each time escapes "the same failures twice".
- Jev is not used in operation: dialogs are cancelled by rule, and the
  escalation model is fixed to `deepseek-v4-pro` in `Decider::default`.
- No testing knowledge persists across objectives; activity lines are kept
  in memory only (300) and show no diffs.
- Worktrees (`base`, `verify`, `trial`, `work`, instances) and branches are
  never removed (16 worktrees registered now); old builds are never pruned;
  the Orchestrator's builds and the Studio's are not serialised.
- Attempts (4) and hours (6) cannot be set; model calls per role are fixed
  (lead 80, implementer 160, reviewer 60, evaluator 80).
- Test instances cannot start in a stated condition (no `--recovered-from`),
  which made objective 3 of W11.7 unmeetable.
- Credentials: every runtime session sets `CLAUDE_CONFIG_DIR` to the
  runtime's own folder and passes an explicit key, so the machine's
  claude.ai login is not read today, but nothing checks the source the SDK
  reports (`apiKeySource` is forwarded and ignored); Settings' runtime card
  still says "Anthropic only". The reference machine has a claude.ai login
  (Max) and no Anthropic API key, Console profile or cloud credentials; on
  2026-10-04 the Operator added their own subscription token
  (`CLAUDE_CODE_OAUTH_TOKEN`) for Agentique's use (C-54 amended), and a live
  check through the pinned SDK answered on `claude-opus-5-5` and
  `claude-sonnet-5-5`.
- Prices: `claude-opus-5-5`'s cache read is $0.40 in the table (it is $0.20);
  `claude-sonnet-5-5` is missing.
- Local checks on the baseline: `python tools/check_architecture.py` OK (13
  crates, 12 parts, 31 dependencies); the companion's 31 tests pass (Node
  22.11). The workspace's Rust checks passed in CI on `388b5dba`.

**W12.3 Models per role and credentials** (merged in #110). Settings › Agents names, for the lead, implementer, reviewer,
evaluator, explorer, escalation and typed decisions, a model from the
capability table (`agents.<role>.model`), its effort among the levels that
model offers, and a fallback with its effort; the defaults are C-54's
(`settings.rs`, `AGENTS`). The Assistant keeps `assistant.provider`,
`assistant.model` and `assistant.effort`; its Anthropic default is now
`claude-sonnet-5-5`, and Settings show its route and why when it is not the
configured one. `agq_orchestrator::models::resolve` takes each role to its
own model when its provider has a credential Agentique may use for that
kind of role, else its fallback with the reason, else none; a credential
store that cannot be read is one candidate's problem, so a key in the
environment still counts. A role the objective needs with no model is named
with what is missing, and the objective does not start: the lead,
implementer, reviewer and evaluator always; the explorer, escalation and
typed decisions only for an objective that explores (W12.5 adds that flag;
until then they are recorded as without a model, never a reason not to
start, so an Operator with only an Anthropic key starts objectives as
before). Anthropic has two
credentials (the Operator's decision, 2026-10-04): an API key, billed per
token, and the Operator's own Claude subscription token from
`claude setup-token` (`CLAUDE_CODE_OAUTH_TOKEN`, or the Credential Manager
entry `agentique:anthropic-subscription` set in Settings › Providers ›
Anthropic), within the plan's limits. The token works only in the Claude
Agent runtime: the session roles use it (preferred when both exist,
`providers.anthropic.credential`), while roles that call their model
directly (escalation, the explorer) and Agentique's own loop need a key and
otherwise fall back with that reason. On the reference machine the defaults
resolve as configured: lead and reviewer on Opus 5.5 and implementer and
evaluator on Sonnet 5.5 through the subscription, the explorer on
`deepseek-flash`, decisions on Jev, and escalation on `deepseek-v4-pro`
because Opus 5.5 there would need an API key. The objective records each
role's model, effort, credential kind and source, who pays and any
fallback's reason, and the roles it does not need that had no model
(`objective.json` stays format 1: optional fields, read by the previous
build's reader as a test shows); a continued objective keeps them (one
whose record has none, saved by an earlier build, gets them from the
current Settings, and the activity says so), and a credential that is gone
stops the session instead of moving it to another. The Orchestrator's
runtime factory builds every session of a role on its record; spend is kept
by role and by model within it, so SDK subagents' models show (usage on the
subscription is priced at API rates, shown as API-equivalent, and the USD
budget applies to it; a dated Claude snapshot such as
`claude-haiku-4-5-20251001` is priced as its model). `models::decider`
builds typed decisions from the `decisions` and `escalation` roles (Jev's
model, and the escalation model with its effort) instead of
`Decider::default`, and `models::with_deciding` gives W12.4's exploration
its `Deciding` from the `explorer`, `decisions` and `escalation` roles in
one call; nothing calls them yet: W12.5's exploring objectives use them. A session gets exactly one credential; the companion holds the prompt
until the SDK has answered its start (within 60 s, else a runtime failure
with its cause) and gives it only when both sides of the SDK's report are
that credential: for a key, `apiKeySource` `ANTHROPIC_API_KEY` and no
token; for the token, `tokenSource` `CLAUDE_CODE_OAUTH_TOKEN` on
`firstParty` and no key (the bundled Claude Code reports the token whenever
its variable is set, yet uses a key or an `apiKeyHelper` instead when one
is in effect). The flag tier of the SDK's settings, which a project's
settings cannot override, blanks every credential the session was not
given (`ANTHROPIC_AUTH_TOKEN`, the other of the key and token, the cloud
switches) and the credential helpers, and a project's `.claude/settings.json`
(and `settings.local.json` when loaded) that brings a credential of its own
or cannot be read keeps the session from starting, with the reason. One
hole remains, stated here: a key session whose project settings are
changed while it runs (by a command, since file tools may not write
`.claude/`) to put another key in `ANTHROPIC_API_KEY` would report the same
source; the next session refuses those settings, and the Orchestrator's
path gate refuses a change to `.claude/` unless the objective names it. The
companion also checks the init message again, reports the source in
`init`, ends a session at a Claude plan's usage limit without moving to a
key, and passes a spend ceiling (`maxBudgetUsd`, what is left of the
budget) on Anthropic's own API. SDK subagents stay on the session's
endpoint and credential and, unless they name a model, run on the role's
(`CLAUDE_CODE_SUBAGENT_MODEL`, pinned in the flag tier); their usage shows
under the role. Settings and the Objectives panel draw from the credentials
as last read, so drawing reads no store: they are read, with Node.js looked
for, on a thread of their own when the Studio starts, when Settings or the
Objectives panel opens and when a key or the token is saved or removed (the
next tick makes the runtime choice again only when something changed), and
read at once when an objective starts; the login probe also runs on its own
thread beside other setup work, so none of it blocks the window.
`main.ts --login` runs the SDK's Claude Code binary's `claude auth status`
with the Operator's configuration and keeps only `loggedIn`, `authMethod`,
`apiProvider` and `subscriptionType`; Settings say the claude.ai login is
not used and why. Prices: `claude-opus-5-5` cache reads $0.20 (were
$0.40), `claude-sonnet-5-5` added; Opus 5.5 defaults to effort medium,
Sonnet 5.5 to high. Measured live on 2026-10-04: the probe reported
`claude.ai` (Max); a session on DeepSeek's endpoint (`deepseek-v4-pro`,
effort max) passed the check with `ANTHROPIC_API_KEY`; a development
session on `claude-sonnet-5-5` on the subscription token passed it with
`CLAUDE_CODE_OAUTH_TOKEN` (`firstParty`), and its Bash command saw neither
the token nor a key. With fake credentials and no prompt (nothing sent to a
model), the real SDK reported a project's key, `ANTHROPIC_AUTH_TOKEN`,
token or `apiKeyHelper` beside the given credential, and none of them once
the flag tier blanked them. The general-purpose and Explore subagents of a
`claude-sonnet-5-5` session on the token reported only
`claude-sonnet-5-5`. Anthropic with an API key is not tried (no key).

**Work items**

| Item | Pull request | State |
|---|---|---|
| W12.1 Direction and self-model | #108 | C-54, Scenario K, §4.16, Stage 12, the decision log naming the locked parts before they change; the self-model's contracts (`Orchestrator`, `Studio`, `ClaudeAgentRuntime`, `Assistant`, `Providers`, the objective and control items) and four requirements (`FindingsReproduce`, `DefectShownBefore`, `ChildWorkBounded`, `OnlyGivenCredentials`); identities reconciled by `Project::open` (only new elements got ids) |
| W12.2 Control interface, complete and observable | #111 | Merged (below): tool inputs checked as declared, every interactive control observed with a readable label and an `operatorOnly` mark, title-bar buttons that answer the operating system's clicks, one agent per window, observer mode with Stop, the trace's why, goal and holder, the model's digest; with the coordinator's additions, test instances where agents also use the Conversation and undo, keys only from the environment there, and refusal kinds. Tested: unit tests, three control journeys on real windows, a real-input title-bar check |
| W12.3 Models per role and credentials | #110 | Merged: Settings › Agents, each role resolved and recorded before an objective starts, its sessions on its own model, effort and credential, spend by role and model, typed decisions from the `escalation` and `decisions` roles; the Claude subscription token as Anthropic's second credential; the companion's credential check and `claude auth status` probe; Claude 5.5 prices |
| W12.4 Exploration and testing knowledge | #112 | Built, not yet in cycles (W12.5): `explore` (the explorer's run behind an `Instance` boundary: a test instance started fresh from a copy of the start project inside its own folder, or a stand-in; the actions valid there that the observation offers to agents, fields with fixed input classes and the Conversation's composer with fixed request classes; the rules, Jev among the rules' best eight, the explorer's model with its answer checked, or Jev escalating; recovery from stale refusals, dialogs in the way, dead ends, exits and hangs), `findings` (the checks `answers`, `offered-acts`, `readable-labels`, `undo-restores`, `dialogs-close`, `action-time`, `no-internal-error`, `turn-ends`, `turn-stops` and the explorer's `expectation`; identities normalised; `replay`, `reproduce` by two replays, `reduce` within a bound) and `knowledge` (`testing/<project>/knowledge.json`, format 1, atomic, bounded); `decide` asks one typed question for dialogs and exploration, the dialog decisions unchanged; fixed exploration tasks, tuning and held-out (`tests/fixtures/exploration.json`); measured live, preliminary (below) |
| W12.5 Exploration in cycles, stronger gates, bounds, a continuing loop | #113, #114 (records), #115 | Merged (below): Explore and Reproduce before Propose, the finding's replay as a frozen criterion, evidence on the base, evaluation of user-facing changes, failure identity, budgets of steps and calls, stated conditions, a loop that continues through adoption and recovery, bounds on worktrees, builds and branches; the Orchestrator's half of W12.6 (delegation, message routing) with it |
| W12.6 The Conversation as the one window: threads, directives, delegation | #116 | Merged; the Studio's side (below): one set of objective commands for the Conversation, the Objectives panel and the palette; one start form (in the Conversation from the message or the Assistant's `propose_objective`, else in the panel) with explore, budgets, permissions and each role's model; the objective's thread in the Conversation in time order (messages with where each went, directives with their recorded status, results, events, tool calls folded with diffs, children nested, directives streaming at the observer speed); replies through the composer addressed explicitly; the panel as a dashboard; agents operate it in test instances (`--assistant-stand-in`), never in the Operator's window. The `delegate` tool, child objectives and the routing of messages are W12.5's |
| W12.7 Proof | — | Not started; folded into Stage 13's proof, W13.7 (C-55) |

**W12.4 measured, preliminary** (live, 2026-10-04, on `stage12/explore` at the code of `2e56d6dc`, as rebased onto `c563d23a`; a debug Studio built from that branch, before W12.2's control-interface changes; Jev `jev-1.13.0` through TypeSafe AI with threshold 0.6 and a 4 s deadline; the explorer's model and the escalation both DeepSeek's `deepseek-flash` at effort `low`, to compare like with like; results kept outside the repository)

Three fixed tasks (`crates/orchestrator/tests/fixtures/exploration.json`: t1 for tuning, the history and a checkpoint, from the URL shortener; h1 and h2 held out, the requirements and the scenarios of the screening sample), each way from the same fresh start and seed, 20 steps each, every finding replayed twice and reduced within six replays. Useful coverage is the coverage keys new to an empty testing knowledge; progress is the goal's areas an observation showed; unwanted actions are refusals (the Operator's own, stale) and actions that ended the instance. Latency is per decision, failed calls included:

| Way | Split | Useful coverage | Progress | Unwanted | Findings (reproduced) | Latency p50 / p95 | Cost |
|---|---|---|---|---|---|---|---|
| Rules | tuning | 20 | 2 of 2 | 0 | 0 | 0 / 0 ms | $0 |
| Rules | held-out | 40 | 3 of 4 | 0 | 0 | 0 / 0 ms | $0 |
| Jev | tuning | 20 | 2 of 2 | 0 | 0 | 241 / 405 ms | $0.0007 |
| Jev | held-out | 40 | 3 of 4 | 0 | 0 | 228 / 262 ms | $0.0015 |
| Model | tuning | 20 | 2 of 2 | 0 | 1 expectation (1) | 3.3 / 19.8 s | $0.040 |
| Model | held-out | 37 | 4 of 4 | 0 | 0 | 6.3 / 30.7 s | $0.106 |
| Jev, escalating | tuning | 20 | 2 of 2 | 0 | 0 | 3.8 / 19.1 s | $0.037 |
| Jev, escalating | held-out | 40 | 4 of 4 | 0 | 1 expectation (1) | 5.6 / 34.6 s | $0.117 |

Twelve runs, 23 minutes, $0.30. Jev was confident in 1 to 4 of each run's 20 decisions, so the Jev way mostly fell back to the rules and the escalating way escalated 15 to 19 times a run, at about the model's cost and latency. Only the model ways reached h1's Requirements panel. The two findings are expectations the model stated and the Studio did not meet (the command palette was to open as a dialog, but an observation shows it as `palette`; clicking the row “Scenario ScreeningTimesOut” was to put that text in the selection, which lists elements by name): both reproduced, both more likely wrong guesses than defects, which is a later judgment. No invariant failed in these runs; an earlier run of the rules (other coverage keys, another seed) found that the Objectives panel's three fields are labelled with their machine ids (`objective-intent`, `objective-usd`, `objective-cycles`), each reproduced and reduced to one step (opening the Objectives tab). What the runs could not do on today's Studio: the undo check (undo is the Operator's; the run says so), and the Conversation (nothing is marked, so it stays the Operator's by place; with W12.2's marks a test instance offers its composer). Tuned on this Studio, in the checks only: coverage keys name the region, not the panel shown; a dialog's confirm names the inputs it confirms; a fill's budget adds 100 ms a typed character (a 300-character fill takes 9 s in a debug build); an empty `input` for a button is no input. Preliminary: three tasks, one seed, one run each; the escalation role's own model is not tried. The code changed after this measurement (the Conversation where a test instance offers it, `a5588318`; the undo check waiting for a dialog, `00ddf6d7`; the answer to the independent review, `0c5c0502`); on that last code, t1 run again by the rules and the model (20 steps each) gave the same coverage (20 and 20 keys) and progress (2 of 2), no unwanted action and no finding, the model at $0.054 (p50 5.7 s, p95 34 s), and the Conversation left alone (the run is not allowed to send requests by default). The rest of the table is not measured on the final code.

**W12.2 built** (branch `stage12/control`, 2026-10-04; merged in #111;
nothing here says the Operator accepted anything).

- **Tool inputs checked as declared** (`assistant/src/tools.rs`): the checker
  knows every JSON Schema type (`integer` is a whole number within 64 bits,
  `7` or `7.0`), checks `additionalProperties` given as a schema (`features`,
  `values`), and says what was expected and what was given ("`input.observed`
  must be an integer (screenRevision from the observation this action is
  based on); got the string "7""). `check_definitions` runs every tool's own
  minimal and full example through the checker and refuses a keyword the
  checker does not check; the Assistant's, the worker's and the worker
  toolset's tools pass it. `minimum`, `maximum`, `anyOf` and `oneOf` are not
  used. `act_in_app` takes an optional `goal`.
- **Every interactive control observed with a readable label:** the palette's
  rows (`palette-<command>`, `palette-element-<id>`, `palette-block-<ref>`,
  role `option`, the highlighted one `selected`) and its search field
  (`palette-search`, which now takes an agent's typing: the palette's text is
  not remembered as an agent's commit, since what runs is chosen by the Enter
  or click that follows), every switch (role `switch`, value `on`/`off`), the
  Inspector's Name, Multiplicity and Docs (their commits are the agent's when
  it typed there), the outline filter, Settings' text and key fields (a key's
  value is never observed), the title bar's search and window buttons, menu
  items (as controls of the region that opened the menu), `status-problems`,
  the Inspector's type matches; the Objectives panel's fields keep their ids
  and read "What should Agentique improve in itself?", "Spend budget (USD)",
  "Improvements" and "Message to the agents", and its switches show their
  text. The label rule (`tests/control_journey.rs`) found nothing on the
  welcome screen, the New project and Checkpoint dialogs, the Surface, the
  Inspector, the palette, Settings, the Objectives panel and a test
  instance's Conversation.
- **`operatorOnly`** on controls and commands, from the rules that refuse an
  agent's action (the key appears only when true; the window's buttons carry
  it on every screen); the palette's and menus' Operator-only commands, the
  Inspector's Lock and the Run panel's trust button are now refused up
  front too.
- **Title-bar buttons answer operating-system clicks.** The cause was as
  recorded in the baseline: GPUI answers `WM_NCHITTEST` with the first
  window-control area under the pointer, the bar's drag area. The window now
  moves by the bar's empty stretches only. Checked with real operating-system
  input (`SendInput` at the bounds an observation gave; display scale 1.0,
  Windows 10), on the same build with and without the fix: before, the
  Settings gear, Maximize, Minimize and Close did nothing (7 of 11 checks);
  after, all 11: the gear opens and closes Settings, a drag on the empty bar
  moves the window by exactly the drag (140, 90 px), a double-click maximises
  and restores, and the three window buttons work. Injected and physical
  mouse input take the same path through `WM_NCHITTEST`, differing only in a
  flag GPUI does not read; other display scales, a second monitor,
  Windows 11's snap flyout, touch and pen were not tried.
- **One agent per window:** the first agent to act holds it until it sends
  `release` or has been idle for 30 s; another agent's action is refused with
  "the window is in use by <agent>; act in your own test instance, or wait"
  (kind `held`). Observing and waiting never are, nor the Operator's input;
  the Orchestrator's own steps by rule through the endpoint (agent
  `orchestrator`) are the supervisor's and pass a hold, so its cancelling of
  a dialog between an explorer's actions, and its setup actions before an
  evaluator's, keep working.
- **Observer mode:** `control.speed` (Settings › Appearance; `observe` by
  default) or `--control-speed`: at `observe` typing goes in about 12
  characters a second and the target is ringed and labelled for 300 ms before
  a click; at `observe` and `fast` clicks ripple and scrolls show their
  direction; the agents chip shows the action in progress, then its outcome
  (agent, goal, decision, done or refused with the reason) for six seconds,
  and gains Stop (the Operator's, like Pause): the action in progress ends at
  once, those waiting are refused and agents stay refused ("stopped by the
  Operator") until Resume. Pause and Step take effect between typed
  characters. Nothing is drawn, and no frame asked for, while no agent acts.
- **Trace:** every event has the agent's `why`, its `goal` and who held the
  window.
- **`project.digest`:** 16 hex digits of the SHA-256 of the model's printed
  text, independent of revisions, ids and saving, worked out once per model
  revision and only for a full observation (`detail: "full"`) or when asked
  (`digest: true`): 143 ms at 10,204 elements in a debug build after a
  change, 1 µs otherwise (not measured in release).
- **Test instances** (the coordinator's addition, the Operator's direction of
  2026-10-04): with `--test-instance`, agents may also use the Conversation
  through the real input handlers (focus the composer, type, send or answer,
  stop the turn, open cards, scroll) and undo and redo; steering the turn,
  retrying, editing, a new conversation, the model, Settings, locking,
  appearance, approvals and the agents chip stay the Operator's. Keys come
  only from the environment there (`agq_providers::keys::without_store`;
  Settings › Providers says so): on this machine, whose credential store
  holds a DeepSeek key, a test instance reports a missing key. The
  observation's `conversation` gains `lastMessage`, `toolCalls`, `notices`
  and `error`; a wait can take `conversationIdle`; every refused action has
  a `kind` (`operator-own`, `stale`, `gone`, `disabled`, `unavailable`,
  `held`, `stopped`, `expired`, `invalid`, `timeout`, `failed`) and
  `conversation.usd` estimates the conversation's spend. The Orchestrator
  does not pass `--test-instance` or `--control-speed` yet.
- **After the independent review:** a press still down when Stop comes (or
  when an approval dialog has opened) is let go away from its control, as
  the agent's input, so it becomes neither a click nor a drag; stopping the Assistant
  ends its action in progress; in a test instance, the Assistant's turn an
  agent's message started acts within that agent's window hold, and the
  Assistant holds nothing once its turn ends; `--test-instance` refuses to
  start without a session of its own; a Stop the Operator gave is lifted
  only by the Operator's Resume; the Conversation's text (messages, replies,
  thinking, a message added to a running turn) is observed only in a test
  instance (in the Operator's window no value of the Conversation's is
  observed, and its items are named "Message 2 from you", "Reply 3, part
  2", "Tool call 4", "Tool error 4", "A question for you"); the controls
  that are the Operator's by id are one table, read by the refusal and the
  mark alike; buttons and switches report their focus, so typing into a
  focused Operator's field and Space or Enter on a focused Operator's
  button or switch are refused up front; the Assistant answering the
  supervisor's own message takes no hold; the brand and the project's name
  drag the window too.

Measured for W12.2 (2026-10-04, the reference machine, while two other
agents built in parallel):

- **Checks:** `cargo fmt --check`; `cargo clippy --workspace --all-targets
  -D warnings`, and for `agq-studio-native` with `--features automation`;
  the tests of `agq-studio-native` (149 and 1, 5 and 3 ignored),
  `agq-assistant`, `agq-orchestrator` and `agq-providers`;
  `python tools/check_architecture.py`; `control_journey` on real windows
  (3 of 3: the agent journey, observer mode, a test instance).
- **Reference run** (release with `automation`; the baseline `96774964` built
  the same way and run interleaved, alternating which went first): warm start
  278–379 ms (baseline 273–365; budget 400); 1k pan p95 6.3–7.2 ms (baseline
  6.3–6.6; budget 8.3); 10k pan p95 17.7–24.0 ms (baseline 19.1–22.2; budget
  16.7) and zoom 14.8–22.1 ms. The 10k budget was missed by both builds
  alike, with frame intervals as long in the steady phase, which has no
  input at all; W11.7's 16.3 ms was measured on a quieter machine. It is not
  met here and is to be measured again on a quiet machine.

**W12.6 built, the Studio's side** (branch `stage12/conversation`,
2026-10-04, on `main` at `813068e4`; merged in #116; the Orchestrator's side,
the lead's `delegate` tool, child objectives and the routing of messages,
is W12.5's; nothing here says the Operator accepted anything).

- **One set of commands** (`commands.rs`): `start-objective` ("Start as
  objective…"), `message-objective` ("Write to the objective"),
  `pause-objective`, `step-objective`, `resume-objective`,
  `stop-objective` and `continue-objective`, in the palette, behind the
  Conversation's buttons and the Objectives panel's alike; all are the
  Operator's (`control::OPERATORS_COMMANDS`), except that in a test
  instance an agent may address the objective and reply in its thread.
- **One start form** (`objective_form.rs`), drawn above the composer when
  the message (Ctrl+Enter or "Start as objective") or the Assistant's
  proposal opened it, else in the Objectives panel: the intent, whether it
  explores (three improvements by default when it does), the spend,
  improvements, attempts, hours and exploration steps, merge and adopt,
  and each role's model with its fallback, credential and who pays (the
  explorer, escalation and typed decisions needed only when it explores).
  `start_objective` checks the budgets (`Budgets::check`; a record whose
  budgets fail it does not continue either), resolves the models with
  `explore`, records `Objective.explore`, and stays the Operator's own.
- **The Assistant proposes** (`propose_objective { intent, explore?,
  budgets? }`, in `agq-assistant`): the Studio opens the same form with
  the proposal and answers that nothing started; with an objective not
  finished it answers "Not shown". The tool cannot start anything.
- **The thread in the Conversation** (`conversation_view/thread.rs`): the
  objective shown (the running one, else the newest the Operator started,
  finished or not; a test instance shows its recorded one and never runs
  it: starting and continuing are refused there) and its
  child objectives are read from their records once, from the tail (the
  latest 2,000 entries each, `Store::thread_last`), then followed as
  `Event::Thread` and `Event::Changed` arrive. Its rows go among the
  conversation's entries in time order: the time each entry was added is
  kept beside the conversation with the objectives started while it was
  open (`projects/<folder>/conversation.objectives.json`, a new app-data
  file; `conversation.json` and its format 2 are unchanged), and rows
  never go before an entry an earlier build added. A conversation shows
  the thread of an objective started while it was open that still goes on
  or ran in this Studio; otherwise the objective in one line with the
  Objectives panel a click away (a new conversation starts empty).
  The Operator's messages ("you → the objective", with a chip saying where
  each went: "to the implementer, at its next tool call", "waits for the
  lead's next turn", or "the objective's intent"), directives (author →
  recipient with its model, a status chip read from the record: running,
  done, failed, stopped, refused with the reason; a child's focus and
  budgets), results and Agentique's events are drawn differently; tool
  calls fold under their step (`under`, or the entry before them in older
  threads) as "N tool calls", each one's diff (removed and added lines
  coloured) or command line a click further; a child objective's thread
  is nested under the directive that started it; a directive that arrives
  while shown streams in at the observer speed (`control.speed`: about 30
  characters a second at `observe`, 240 at `fast`, at once at `instant`,
  each within six seconds). The thread is never in the Assistant's
  conversation, so no model receives it. Rows are worked out again only
  when the objectives, what is expanded or the conversation change, keyed
  by their fields; a directive streaming in updates only its own row, and
  only the Conversation is drawn again for it.
- **Steering in the thread:** the composer addresses the Assistant or the
  objective ("To: the Assistant" / "To: the objective", a line above the
  message naming the objective); only the Operator's click, Ctrl+L ("Ask
  the Assistant"), "Write to the objective" or a step's Reply switches it.
  A reply goes through `Studio::message_objective`, the panel's message
  field's path too: to a running objective's handle (the Orchestrator
  records where it went, `ThreadEntry.to`), or, for one waiting to
  continue, into its thread with `to: "lead"` for the lead's next turn; an
  objective that has ended takes no message and cannot be addressed. An
  open question of the Assistant's is answered first, whomever the
  composer addresses.
  Messages go to the objective the Operator started; a child is steered
  through it. The Studio's own entries (notes, replies, the intent written
  for an earlier build's record) pass through `thread::redacted` with the
  configured keys and token. The objective's bar above the list has its
  phase, spend and children, and Write to it, Pause, Step, Resume, Stop and
  Continue.
- **The Objectives panel as a dashboard:** the record (phase, budgets,
  each role's model and spend), the tree of child objectives (who asked,
  state, spend of budget), the same commands, the message field, the
  latest eight steps and "Show its thread in the Conversation"; the start
  form when no objective is unfinished and the form is not open in the
  Conversation.
- **Self-testability:** every new control has a readable label and a
  stable id (`thread-<objective>-<n>`, `thread-fold-…`, `thread-reply-…`,
  `conversation-to`, `conversation-to-objective`, `objective-from-message`,
  `objective-bar-…`, the form's `objective-…`). In a test instance agents
  read the thread with its text, expand rows, reply and switch the
  composer through the real input handlers; in the Operator's window the
  rows read "Directive, entry 3, by lead", without text, and all of it is
  refused. The observation gains `objective` (state, phase, spend,
  children, the start form, the thread's size and last entry, the text
  and intent only in a test instance) and `conversation.addressed`; the
  values of `objective-…` fields are observed only in a test instance.
  `--assistant-stand-in` (a test instance only; refused otherwise) runs
  the instance's Assistant on a scripted stand-in in every build: no
  network, no key, no cost; it reads the model and answers in a line,
  shown as "scripted stand-in (no network)".
- **A scripted Assistant stays scripted:** a runtime given for the process
  (`ConversationPanel::use_given`: the journeys' scripts and
  `--assistant-stand-in`) is never replaced when Settings change or the
  credentials read at start find a key. Before this, since W12.3, the
  background read of the credentials replaced the `a-assistant` and
  `h-library` journeys' scripts with the model of a key in the Credential
  Manager: on the reference machine one such run of `a-assistant` made a
  live DeepSeek call (`deepseek-flash`, estimated $0.005) and failed at
  step 7. A unit test fails without the rule.

Measured for W12.6 (2026-10-04, the reference machine, while another agent
built in parallel and the Operator used the machine):

- **Checks:** `cargo fmt --check`; `cargo clippy --workspace --all-targets
  -D warnings`, and for `agq-studio-native` with `--features automation`;
  the workspace tests (780 passed, 24 ignored; after the review's fixes
  785 passed, 24 ignored, `agq-studio-native` 182 and 1, with 5 and 5
  ignored); `python tools/check_architecture.py`; `control_journey`
  on real windows (5 of 5, two new: the test instance with a recorded
  objective and the stand-in Assistant, and the Operator's window);
  `a-assistant`, `h-library` and `d-daily` (release with `automation`,
  every provider key variable removed from their environment) passed,
  with no spend.
- **Reference run, the Conversation** (release with `automation`, `chat`:
  200 messages, scrolled 180 frames, a reply streamed at about 100 tokens a
  second; two runs each): without an objective, the first frame after
  loading 8.5–8.6 ms (budget 150), scroll p95 6.18–6.20 ms and streaming
  p95 6.53–6.55 ms (budget 8.3), frame CPU p95 2.6–3.5 ms; with a
  recorded objective of 1,981 thread entries (180 steps of a directive,
  eight tool calls with diffs, a result and an event) below the 200
  messages, scroll p95 6.19–6.22 ms, streaming p95 6.46–6.56 ms, frame CPU
  p95 2.7–2.9 ms. Not compared with a baseline build of `main` this time
  (disk); G3 measured scroll and streaming p95 6.23 and 6.25 ms. These
  runs were before the review's fixes, when every Conversation showed the
  thread.
- **After the review's fixes** (directives streaming update only their
  rows): the release build was killed while building (the parallel
  agent's build, and free disk down to 2.5 GB), so it was measured in a
  debug build with `automation` (two runs each; debug frames are slower
  throughout): `chat` without an objective, streaming p95 22.6–24.5 ms
  (CPU p95 20.6–22.1, median 17.3–17.4); with the 1,981-entry objective's
  thread shown (720 rows) and seven directives streaming into it during
  the reply's stream, streaming p95 24.8–27.6 ms (CPU p95 22.6–25.3,
  median 18.8–20.3), scroll p95 23.6–26.0 ms against 23.3–23.9. The work
  per stream step, timed alone in a debug test build on those 720 rows:
  0.5 % of before (0.019 ms against 3.8 ms for all rows and their Debug
  text), and 2.3 ms for a full rebuild with field keys. Not measured in a
  release build after the fixes.
- **Not tried:** a live objective running in the Conversation (it needs
  W12.5's Orchestrator side, and a credential; the thread was exercised
  from recorded objectives and unit tests); the start form drawn in the
  Conversation was not looked at as an image (opening it is the
  Operator's, and no input was sent for the Operator). A child objective
  stopped alone has its control since W12.5's reconciliation (below):
  `objective-stop-child-<id>` on the child's row in the panel's tree and on
  its directive in the thread, the Operator's own (refused to agents like
  every `objective-` control), sending `Command::StopChild` to a running
  objective and settling the records of one that is not running; tested in
  the Orchestrator's stand-in tests and the thread's rows, not pressed live.

**W12.5 built** (branch `stage12/loop`, 2026-10-04; the thread and
directive records (Step 0) and their review fixes merged as #113 and #114;
the rest merged as #115; nothing here says the Operator accepted anything).

- **Explore and Reproduce before Propose** (`run/explore.rs`) when an
  objective explores (`Objective.explore`; the cycle's sub-phase is in
  `Cycle.exploring`, so `phase` stays `propose` and the previous build reads
  the record). The lead plans first: a short session with
  `submit_exploration`, its goal recorded as the lead's directive to the
  explorer, and `delegate` (below); an objective two deep has no planning
  session. The explored build is the cycle's base: the running build when
  its manifest is of the base commit, else a debug build of the base
  checkout whose executable is kept with the cycle. Fixed findings not yet
  replayed in that build go first (one that fails again is a regression and
  is reproduced). The explorer's run then starts in a test instance
  (`--test-instance`, `--control-speed` at the Operator's `control.speed`),
  from a copy of `models/url-shortener` or of `model/`, alternating by cycle
  and exploration, with the recent changes since the last explored commit
  (`git log`, bounded) and successive ways of deciding (Jev escalating, the
  model, the rules; the rules alone when the objective recorded no
  exploring models). A merged build's instance gets the explorer's provider
  key in its environment only, its Assistant preset to that model, so it may
  send requests, whose spend counts as the explorer's; an unreviewed build's
  never gets one. Spend is counted by the role that decided (`decisions`,
  `explorer`, `escalation`). The run goes to the testing knowledge with the
  build and commit the Orchestrator chose (`RunRecord.commit`, read back by
  `last_commit`). Reproduce takes at most three new findings, most severe
  first (`Check::severity`: exit, hang, internal error; a refused offered
  action; an expectation, undo or a dialog; a label; slowness), each
  replayed twice and reduced within six replays; the knowledge keeps what
  became of each. Nothing new reproduced reproduces again, on this base, a
  reproduced finding the knowledge keeps that no cycle fixed and the
  objective tried fewer than twice, and offers it only if it still fails
  here (one that no longer does is recorded as not reproduced); with none, it explores again from another start and way, within
  the cycle's attempts; two explorations in a row with nothing new
  reproduced, and no such finding left, end the objective as "Nothing new
  reproduced", an outcome. A finding records the build the Orchestrator
  explored; a child objective explores its parent's base build.
- **Propose:** the lead's brief lists the cycle's reproduced findings (`f1`,
  `f2`, …: check, steps, reduced steps, evidence, build) and the testing
  knowledge in short; `submit_proposal` names the one it fixes (`finding`,
  required when there are any), and the Orchestrator freezes that finding
  with the proposal (`Cycle.replay`), its replay being the criterion
  `replay`. A proposal naming the part `Studio` without an observation,
  judgment or finding is refused and asked again; an observation criterion
  may name a stated condition.
- **Evidence on the base** (`run/evidence.rs`, requirement
  `DefectShownBefore`): the replay on the base build (its reproduction is
  reused only when the finding's own build is the base build; otherwise it
  runs again there, from the base's start project); command criteria in a
  checkout of the base of their own with the checked commit's new and
  changed test files brought over, evidence only when a failing test's own
  output shows an assertion that failed (cargo: its `---- name stdout ----`
  section, any panic but an `unwrap` on nothing or an error, by its message,
  or an `expect`, by the line it points to; Node: the failing subtest's own
  block, an `AssertionError`; Python: a `FAIL`, not an `ERROR`); a compile
  or load error, another error, no test, a timeout or a run that did not
  finish is `no evidence`, with the reason; observation criteria in test instances of the base build, where
  only the criterion's own expectation failing is evidence (a setup action
  refused, a broken connection, a way not cleared or an instance that did
  not start is no evidence); a judgment, or a base that does not build, is
  no evidence. The test files are recorded with their blob ids
  (`Cycle.evidence`), and an attempt whose test files differ has its test
  runs on the base made again. The gate "the criteria show the defect on
  the base" counts only the frozen criteria and the replay, needs one with
  evidence and none passing, names the commit whose test files the evidence
  was made with, and fails unless they are the checked commit's own; it
  replaces "the criteria fail before the change".
- **Evaluate** whenever a criterion is behavioural, the cycle has a replay,
  or the diff touches the Studio's code (the part `Studio`'s crate links on
  the base or in the change, outside tests; `gates::user_facing`): the
  replay must pass on the change's build, started from the base's start
  project; a user-facing change also gets a 10-step exploration by the
  rules from the base's URL shortener toward the areas it touched, where an
  invariant failure the knowledge did not hold before fails it, and "the
  user-facing change was evaluated" passes only if one of its behavioural
  outcomes ran there; observation criteria share one instance unless they
  name a condition. The change's build is unreviewed: it starts only as a
  test instance with `--assistant-stand-in` and no credential; a build
  without those flags is not started (its criteria are not run, no pass).
  Flags are read from the build's `--help`, run with only what a process
  needs in its environment, read while it runs and ended after 20 s; a
  merged build without the newer flags still starts, and gets no explorer
  key unless it starts as a test instance. A user-facing change whose frozen criteria have
  no behavioural one stops the cycle (a later attempt cannot add one).
- **Failure identity** (`Outcome::failure`): a judgment by its criterion and
  verdict (`Outcome.judged`); a reviewer's request for changes is the
  failure `review`; numbers and long hexadecimal ids normalised.
- **Test instances in a stated condition** (`control::prepare`):
  `recovered` (a builds registry with a build that did not start, and
  `--recovered-from` it) and `with an objective` (a finished objective with
  its thread beside the instance's session, as W12.6 shows one).
- **Continuing and durable:** `Objective.interrupted` marks an objective
  interrupted because the Operator closed Agentique: the run's every save
  after the interruption says so, and the Studio marks the record itself as
  it closes, under the record's lock (`Store::mark_interrupted`); it waits
  for Continue. One handed over to an adopted build (its continuation), or
  running when the launcher started the last known good build after one
  that did not start (`--recovered-from`), goes on by itself and says so in
  its thread; a plain start under the launcher (`--supervised`) is no
  recovery, and neither is the launcher's restart of a build that crashed
  after it settled, which passes no flag (the launcher is unchanged under
  C-54): such an objective waits for Continue. `Objective.resumes` counts
  resumes without progress, and two stop it (`Objective::on_start`, which
  reads only typed fields). Pause is kept in the record and holds across
  restarts. Exploring objectives default to three cycles
  (`Budgets::exploring`, which the start form of W12.6 uses).
- **Budgets:** `Budgets.steps` (an exploration's actions, 20 by default)
  and `Budgets.calls` (model calls per session role; `DEFAULT_CALLS` keeps
  80, 160, 60 and 80), checked by `Budgets::check`.
- **Bounds:** a cycle that ends removes its worktrees and folders but, when
  it failed or was interrupted, its `work` worktree, of which the three most
  recent across objectives are kept; each is removed by its name (never a
  repository-wide prune, which would touch the Operator's own). The `work`
  worktree goes before the merge, which passes `--delete-branch`; the local
  branch is deleted if the host's merge did not, and the thread says what
  became of the branch here and on the host (`git ls-remote`). One build at a
  time: `builds::lock` (the builds folder's `build.lock`), taken by release
  builds (Settings › Builds and the Orchestrator) and debug builds. Test
  instances' folders are removed after use.

**W12.6, the Orchestrator's half: the delegation backend** (branch
`stage12/loop`, with W12.5; merged as #115).

- The lead's `delegate` tool (`{ instruction, focus?, usd, steps }`), in
  planning and in Propose of an objective that explores, checked by the
  Orchestrator (`run/children.rs`, requirement `ChildWorkBounded`): a budget
  above nothing and within what is left once the lead's own spend in its
  session is counted, steps within the parent's, six minutes of the time
  budget left at least, at most two deep, one child at a time, three in a
  turn of the lead and five in a cycle; the objective's budgets are checked
  again after each child. A refused one is recorded as
  a refused directive with the reason and shown at once; an accepted one is
  recorded as the lead's directive to the child (with its budgets and
  permissions: explore only, no push, merge or adopt), the child objective
  (`<parent>-c<k>`, one deeper, `requested_by` the lead, the parent's models)
  created once under the journal, and the lead's turn ends. The child runs
  inside the parent's run, on its Handle (its thread entries with its own
  objective id, `Event::Changed` for its record): it plans, explores and
  reproduces, then ends with its result (findings, coverage, spend), which
  is recorded on the directive, in its thread and in the parent's as a
  result `to` the lead. Its spend is added to the parent's, once: the
  directive keeps what was counted (`Directive.counted`), so after a restart
  only the rest is added; its reproduced findings join the parent's cycle. A child interrupted with its parent goes
  on first when the parent does. `Command::StopChild(id)` stops one child
  (also one about to start): its directive ends as stopped with "stopped by
  the Operator", and the lead goes on with that; Stop stops the children
  too. `Store::active` returns only the Operator's objectives.
- **The Operator's messages:** to the implementer at its next tool call
  while its session runs (the thread entry `to` is `implementer`),
  otherwise to the lead's next turn (`to` is `lead`; never the reviewer or
  the explorer); what the implementer's session ended before taking goes to
  the lead, and the thread says so. The lead's sessions open with what waits
  for it (the Operator's messages and children's results after
  `Objective.delivered`), which is then marked delivered, with a thread
  event.

**Reconciled with W12.6** (#116, rebased onto `189fba4b`): the Studio's
start-up decision keeps W12.6's choice of the newest objective the Operator
started and its test-instance branch first, then `Objective::on_start`
(resume, wait or stop); `objective_setup` passes the observer speed, the
credential source and the live Studios; the interim filters of children's
events are gone (W12.6 nests children); a message's chip reads "given to
the lead" once `Objective.delivered` covers it; configured keys are counted
in characters (`thread::KEY_CHARS`).

Checked for W12.5 and the delegation backend (2026-10-04):

- **Tested with stand-ins** (`tests/exploring.rs`: the scripted companion
  `fixtures/explore-companion.mjs` and the stand-in Studio, whose builds are
  checkouts and whose instance has an unlabelled button until a build holds
  the fix): an exploring cycle explores the base build, reproduces and
  reduces the finding to one step, the replay fails on the base (its
  reproduction reused) and a brought-over test fails there too, the change
  passes both, the testing knowledge keeps the run and the finding, and the
  failed cycle keeps only its `work`; two explorations without a new problem
  end the objective; a delegation over budget is refused and recorded, one
  within it runs as a child whose result reaches the lead and whose spend
  counts in the parent's; a child stopped alone; messages to the
  implementer at work and to the lead; after the independent review of
  PR #115: a test weakened in the repair has its evidence made again and
  fails the gate; a fourth delegation in a turn is refused; a child's spend
  before a restart is counted once; a known finding left unfixed is offered
  again and replayed on the new base. Unit tests: stated conditions, flag
  probing, an unreviewed build without the flags not started, failure
  identity, evidence on the base (assertions only; an observation's own
  expectation only; only the frozen criteria count), user-facing files by
  both links, "evaluated" only when something ran, the build lock,
  `Objective::on_start` on typed fields and `Store::mark_interrupted`, the
  delegation bounds, the record of an exploring cycle read by the previous
  build.
- **Exercised live** (a debug Studio built from this branch; windows, no
  key, no model): test instances start as test instances at `instant`; the
  `recovered` one's status names the build that did not start; the `with an
  objective` one has the objective in its app data; an 8-step exploration by
  the rules runs in a live instance; every instance folder is removed; this
  Studio has no `--assistant-stand-in` yet, so it is not started unreviewed.
  This found that an exploration given no spend budget stopped before its
  first step, which the rules' exploration of a change had; fixed.
- **Not tried:** a whole exploring cycle on the real Studio with real
  models, a pull request, merge and branch deletion through GitHub (the
  worktree removed before the merge, the host's `--delete-branch`), an
  adoption that goes on to explore the adopted build, the Studio's own
  start-up decision on a real restart (unit-tested only), and the explorer's
  key in a test instance (no exploring roles with keys were used).

## Stage 13: Agentique models, simulates and evolves systems, itself included (C-55)

Status: **in progress.** Directed by the Operator on 2026-10-09 (C-55,
ROADMAP §6.10): the purpose is recorded once (ROADMAP §1.1); the KerML/SysML
foundations grow where engineering scenarios need them, each complete on the
whole path; the self-model states the purpose's obligations and models the
autonomous lifecycle; a second system exercises the same abstractions; the
autonomous loop is aligned with the purpose; and Stage 12's proof runs with a
modelling objective. Nothing here says the Operator accepted anything.

**Baseline** (`main` at `f3f313b0`, CI green; the installed build
`202610041807-f3f313b0fb` is current and last known good; the Operator
reviewed this `main` before giving C-55, so the tag `approved-baseline` marks
it). Read against the code and the model before any change:

- *Subset* (`docs/subset.md`): `ref part` and every `ref` with a kind
  keyword, `assume`/`require` constraints, `calc def`, `constraint def`,
  `state def`, `action def`, `perform`, `flow`, `allocation`, individuals and
  units are excluded or unsupported; requirements carry a subject, doc text,
  attributes and `satisfy` only, so no requirement can be evaluated.
- *Evidence:* the Requirements panel headlines "N of M satisfied", counting
  `satisfy` declarations (`studio-native/src/panels/requirements.rs`), so a
  claim is shown as satisfaction; the Inspector lists the scenarios that
  verify a requirement with their newest results (`panels/evidence.rs`).
- *Self-model* (`model/Agentique.sysml`): no statement of the purpose; the
  Orchestrator, the part that runs autonomous work, has ports and a contract
  but no behaviour and no scenario, so the autonomous lifecycle is described
  only in prose; `connect orchestrator.control to studio.control` says the
  Orchestrator drives its own Studio, while it drives test instances (other
  builds of Agentique); the Studio holds one state machine for three
  workflows; each logical part owns its crates as `Crate` parts (its
  allocation to code), and `ClaudeAgentRuntime` has no crate.
- *Loop* (`crates/orchestrator`): a proposal names its parts, plan and
  criteria, but not the requirement it serves; nothing compares what a cycle
  changed with what it named, or the change since a human-approved state;
  `ROADMAP.md` is not a protected path; findings have no disposition, so an
  explorer's mistaken expectation can be proposed again (W12.4's measured
  runs found two expectation findings that were "more likely wrong guesses
  than defects").

**Work items**

| Item | Pull request | State |
|---|---|---|
| W13.1 Direction | #117 | Merged |
| W13.2 Referential usages | #119 | Merged (below) |
| W13.3 Requirement constraints and evidence | #121 | Merged (below) |
| W13.4 The self-model | #118, #122 | Merged (below); the purpose's obligations in review in #122 (below) |
| W13.5 Alignment in the loop | #120 | Merged (below) |
| W13.6 A second system | #122 | In review (below) |
| W13.8 The start of an objective (the Operator's amendment of C-54, during W13.7) | — | In progress (`stage13/start`) |
| W13.7 Proof | — | Started: blocked by W13.8 (below) |
| W13.7 repairs: exploration (E1 what is explored, E2 progress and time, E3 engineering questions) | #130 | E1 in review (below) |

**W13.1** (#117): the purpose recorded once (ROADMAP §1.1) with its
protections named; C-55 and Stage 13; the claims of §4.14; alignment in
§4.16; `AGENTS.md`, `README.md` and the one-truth tables refer to §1.1. The
self-model holds the requirement `purpose` (no `satisfy` yet: the
Requirements panel would show a declaration as satisfaction until W13.3),
locked on the Operator's C-55 instruction. External agents change a model
through `apply_changes` without a window (`README.md`, "Changing a model
without a window"); the tools now also address an element without a name by
the name they show it under (`Owner::(connect a to b)`), so agents can change
or delete connections and `satisfy` relationships.

**W13.2 Referential usages** (`stage13/ref-parts`, PR #119; nothing here
says the Operator accepted anything). `ref part` and `ref item` on the whole
path: the language core (a `referential` flag; parse, print in the standard
prefix order, link, the validity rules above and deviation 19), the System
State (`Property::Referential`), the Library (copies keep the flag; no value
or override is given through a reference), the Assistant (`apply_changes`
`"ref"`, `read_model` and `inspect_behaviour` say what a reference refers
to or that it is not bound), `explain_element` and the Inspector ("Refers
to …", a Usage switch between composite and reference, fixed for directed,
`end` and package-level usages, Owns marking references), the Surface (a
dashed card edge, no new colour), and model execution (a bound reference is
the instance it refers to; one not bound has only its ports, and a message
reaching it stops the run with `missing-stand-in` unless a stand-in
answers). One rule decides what is referential everywhere: a usage
referential itself (`ref`, no kind keyword, directed, `end`, or owned by a
package) whose redefined part and item usages are all referential, since a
redefinition has the values of what it redefines. After an independent
standards review: a `ref` redefinition of a composite part is composite;
an inherited binding is kept and never changed in a redefinition (of a
reference, or of a composite part or item); self and
mutual bindings, and `ref part … = new T()` (unsupported), are reported;
whether a connection passes items inward is decided by what a reference is
bound to. Tested: language (13 tests), System State (2), Library (1),
Assistant (1), explain (2), simulation (14: both paths reach the one bus,
`= power.bus` and a keyword-less binding through another reference, an
inherited binding, a reference with features of its own, a `ref item` to a
part, a binding that reaches no running part, reading and setting through a
reference not bound, a message into one, a stand-in answering for one or for
a bound reference, a reference bound to one not bound answered by one
stand-in, bindings to themselves), scene (1). Workspace: 842
passed, 25 ignored; CPU-side budgets in release passed (scene build at 10k
1.78 s, an edit to the Surface at 10k 75.9 ms, a System State edit at 10k
45.4 ms). Not verified: the Studio visually (no window was opened), the
reference run, replay and live modes with references. Limits: the Library's
block summary counts a reference as contained; a part's own state machine
sends only through its own ports; `bind` stays unsupported; a reference with
a multiplicity above 1 binds one instance; attributes may still override a
value given where they are declared (only bindings of references are
refused).

**W13.5 Alignment in the loop** (`stage13/alignment`, PR #120; nothing here
says the Operator accepted anything). As the §7.6 entry of 2026-10-10
describes: proposals name what they serve, benefit and cost, checked against
the base's model; traceability and the cumulative change since the approved
baseline (read at the `origin` URL the objective recorded, without
credentials) for the reviewer, who answers both explicitly; the purpose
gate, for projects that declare a root purpose, reading models by the
commits' trees; no tags or remote changes in worktrees, and no git aliases
or includes, releases, tag refs, GraphQL mutations or `gh` without the
network in any session (the Operator's Conversation too); dispositions of
findings with stable ids. After the independent reviews of #120: the purpose
gate and the locked-element gates no longer read the checkout the checks
ran in; the purpose is protected across cycles (moved or renamed, and as
the objective's start and the approved baseline declare it); model read
failures stop the cycle; models without requirements record `serves` as
stated; ambiguous findings are not reproduced again; an unreliable
reproduction is offered again only on another build than it was judged on;
blank review judgments are refused. The refusals cannot close every way to
move a tag: the approved baseline's real control is a GitHub tag ruleset on
`approved-baseline`, recommended to the Operator. Tested with the scripted
stand-ins (`tests/traceability.rs`, `tests/cycle.rs` with cycles that edit
`ROADMAP.md` and the purpose, one while a check deletes `model/`,
`tests/exploring.rs` with wrong-expectation, ambiguous and re-judged
findings, unit tests in `traceability.rs`, `roles.rs`, `gates.rs`,
`knowledge.rs`, `record.rs`, `forge.rs`, and the assistant's `policy.rs`
and `model_tools.rs`), after rebasing on `main` at `ed6a91d9`:
agq-orchestrator 136 passed, 0 failed, 5 ignored; agq-assistant 118
passed, 0 failed, 4 ignored; workspace clippy, fmt and the architecture
check passed. Not verified: live models; the approved baseline against
GitHub (tested with local bare remotes); reversing a disposition by an
Operator command (not built).

**W13.4, first part** (`stage13/self-model`, made through `apply_changes`;
nothing here says the Operator accepted anything).

- *The autonomous lifecycle, modelled and run.* The Orchestrator is its
  deterministic `Driver` and its role agents, nested in it so its lock
  covers them. The Driver's 34 transitions follow `run.rs` as merged with
  W13.5: the lead plans
  (and may delegate, three children a turn) unless the objective is a child
  at the maximum depth; the explorer's run reports what failed, and the
  Driver's deterministic replays decide what reproduced; a child explores
  once and returns; nothing reproduced explores again, and two empty
  explorations in a row end the objective; a proposal whose names do not
  resolve in the model is asked again; the checks on the commit decide the
  evidence on the base and the gates (the purpose's among them, W13.5), a
  failure going to repair and a round without fewer failures ending the
  cycle; the reviewer's rejection goes to repair; merging needs the
  permission, adopting too (a merge without it is done); a build that does
  not take over ends the objective; after a cycle, done or failed, the next
  starts while cycles are left. Folded, as its doc says: Evaluate into Check,
  Try into the build's answer, the same failure twice into a round without
  fewer failures (the model ends some cycles earlier than the code, as
  its doc says). Eleven scenarios run in model execution with the agents
  stood in and the environment answering at the Orchestrator's boundary: an
  objective that adopts and explores again until two empty explorations
  end it; a failing check repaired; a round without fewer failures; no
  defect shown on the base; a reviewer rejecting twice in a row;
  the governing text kept the Operator's; a merge without the permission to
  adopt; a build that does not take over; a fourth delegation in a turn
  refused; a child that explores once and returns; every finding judged
  not a defect (nothing to fix). They check the model's account; the code
  is checked by the tests linked to 19 of the 34 transitions
  (`tests/exploring.rs`, `tests/cycle.rs`, `gates.rs`, `run/children.rs`,
  `record.rs`), W13.5's among them (a proposal serving a part refused; a
  cycle that rewrites the purpose not merged even when a check hides the
  model; a finding judged a wrong expectation). No test exercises a
  rejected review, a merge, its repair or permission, a build, its try or
  a build that does not take over as the Driver records them, a child at
  the maximum depth, or the start and continuation of cycles, so those 15
  transitions are not linked.
- *Corrected after review.* The first version (an independent
  architecture review asked for changes twice) had the lead declare its own
  evidence and purpose verdicts, ended every failed cycle, linked tests that
  did not exercise their transitions and left the new definitions outside
  the lock; it was redone from the code, then reconciled with W13.5 (a
  refused proposal gets two lead sessions; nothing to fix; two rejections in
  a row end a cycle; every exploration is planned). The test instance the
  explorer operates is `ref part testInstance : Agentique[0..1]`: the
  Orchestrator refers to another Agentique, which as a composite part would
  be a composition cycle.
- *One mechanism where the code has one.* The Studio's state machine had a
  second lock question, confirmation and two states for the Assistant's
  changes (14 states, 29 transitions); it now remembers who asked
  (`requester`) and has one change path and lock question for every actor
  (12 states, 27 transitions). The five scenarios of edits and Assistant
  actions pass unchanged, the question naming the Assistant when it asked.
- *Tools.* `apply_changes` gains an effect of several steps, a composite
  action created with its steps, and `set` moving a transition between
  states; with the lookup of unnamed elements (W13.1) an agent can now write
  and generalise behaviour while transitions keep their identities. Before,
  a transition could have one `send` or `assign` only.
- *Traceability.* A dogfood test checks every implementation link: the
  element exists under that name, the path exists, the function is defined
  there. It found two stale links on `main` (a test moved to the
  Orchestrator in W11.5, a test renamed in W12.5), fixed.

**W13.3 Requirement constraints and evidence** (`stage13/requirements`;
nothing here says the Operator accepted anything). Before: requirements had
a subject, doc text, attributes and `satisfy`; the Requirements panel
headlined "N of M satisfied" from declarations. Now: assumed and required
constraints (formal or informal, private if wanted) and subrequirements on
the whole path (language, System State, `apply_changes`, `read_model`),
their evaluation on the modelled configuration of each satisfying feature
(`agq-simulation`, `requirements`: built in the part or part def the
satisfy is written in, members counted whatever their visibility, roll-ups
through feature chains, a `ref part` being the part it refers to (counted
once; unbound: not determined), values worked out once with cycle detection, a
redefinition in a usage winning over its definition; a subrequirement whose
assumptions are not met does not apply; problems in the slice read, parts
or a satisfying feature with a multiplicity other than one, informal or
undetermined values, ambiguous subrequirement names, a subject bound twice
and a requirement that contains itself give "not evaluable", never a pass;
deviations 21 to 23), and the evidence ladder (`agq-implementation`,
`requirements`): declared, calculated, scenarios with their freshness (an
implementation run current only while the code is what it ran; only a
failed check or a stop by the model's own behaviour is a failure, anything
undecided inconclusive), linked tests. The panel's headline counts the
standings of requirement usages, subrequirements with their container ("1
violated by calculation · 1 holds by calculation · 1 holds for some
calculations · 1 only declared · 1 with no evidence (of 5)"), each row
opens its ladder, the Inspector shows the same, and the button says
"Declare satisfied by …". The Assistant's `check_requirements` answers the
ladder as text and, headless, says that kept results and links are not
available. Tested: language, System State, simulation (a mass budget that
holds, is violated with its values, has assumptions not met, is informal or
undetermined; violated, private, ambiguous, non-applying, self-containing
and mistyped subrequirements; contexts and their digest; cycles, division
by zero, enums), assistant, implementation (stopped and empty runs, mixed
calculations) and Studio (a stale implementation pass) tests, and a shared
`ref part` bus counted once: on `main` at `b033dc4f` (the branch's head
before the last change), `cargo test --workspace` 918 passed, 0 failed, 25
ignored; workspace clippy, fmt and the architecture check pass. The
Scenarios panel's chips now classify a result as the ladder does (a run
with no or undecided checks is inconclusive, not failed): the Studio's
tests 187 passed after that change. Not verified: the panel by eye, the
reference run (with the final proof),
a real model calling `check_requirements`. Limits: no units; package-level
constants are not read; scenario runs still work attribute values out
differently (deviation 23); a change to the code alone reaches the ladder
when the model, the links, the kept results or the checks next change.

**W13.6 A second system** (#122; nothing here says the Operator accepted
anything). `models/inspection-charging/InspectionCharging.sysml`: an
inspection drone and its charging station, after the Operator's guide
"SysML v2: From Syntax to Systems" (its Aster example), as text like the
other examples. Its documentation states the boundary (the drone's mass by
assembly, the station's charger, the charging contact; outside: the
authorisation service, which the stations refer to and scenarios stand in,
flight dynamics, electrical protection, current, heat, weather, regulation),
the units (grams and milliseconds; deviation 14) and that nothing in it
verifies a physical drone or charger. Bounded questions and what answers
them (`crates/simulation/tests/inspection_charging.rs`, 17 tests, passing):

- *Composition and sharing.* The flight computer's `ref part :>> supply =
  bus;` is the drone's one bus in two roles (a requirement reading
  `flightComputer.supply.mass == bus.mass` holds at 150 g). The bus is
  counted once because the sum names it once and the controller's mass
  excludes it: a sum that also added the supply counts it twice (6050 g).
  A composite part bound to the bus is reported (`wrong-value`, SysML
  7.6.3). Stated as a limit: a second, composite bus inside the controller
  is not reported, and a sum that does not name it leaves it out (5900 g);
  Agentique does not check that a roll-up covers every composite part. The
  dock refers to the survey drone and the home station and connects them
  with the `ChargingLink` interface (conjugated ports).
- *Calculation on the modelled configuration.* `LaunchMassLimit` (assumed:
  the drone carries a payload; required `aircraft.mass <= limit`) holds for
  the survey drone at 5900 g of 7000 g; the upgraded drone, which states
  only what changed from the survey drone's configuration (650 g avionics,
  2300 g payload), violates it at 7150 g (its `satisfy` declaration is shown
  wrong) and meets the ferry limit of 8000 g; without a payload the limit
  claims nothing (assumptions not met). `BoundedWait` holds at home
  (500 ms × 2) and in the field (2000 ms × 3, exactly 6000 ms), and one more
  field attempt is caught (8000 ms).
- *Behaviour in model execution.* The contacts stay off while the answer is
  pending (checked 50 ms in) and come on only after an acceptance (read from
  the trace); a refusal leaves them off; a lost answer is asked again and
  decided at 500 ms; two lost answers end a home station's attempt at
  1000 ms; the field station, the same `Charger` definition configured for
  a slow link, energises on its third attempt (4000 ms) and gives up at
  6000 ms when every answer is lost, the calculated bound; a plausible
  wrong design (`EagerCharger`, energise first, switch off on a refusal)
  fails `offWhileAsking` while its end state alone would pass. It has its
  own requirement (`quickAuthorizedCharging`), so it never counts against
  the correct station's. Runs are the same run every time (whole traces).
- *Kept apart.* The behavioural requirement is informal: not calculated,
  never shown as holding on its `satisfy`; six scenarios verify it.

Not verified: the model in the Studio by eye; units. Before this stage the
same model text had five unsupported constructs and nothing of its mass or
wait could be calculated. An independent review found the first version's
claim about the bus (that a copy would make 6050 g) wrong for this model,
the wrong design's scenario counting against the correct station, and
end-state checks that could not show ordering; corrected.

**W13.4, the purpose's obligations** (#122, made through `apply_changes`,
the lock of `Purpose` confirmed on the Operator's C-55 instruction).
`Purpose` gains eight informal subrequirements (explicit architecture;
executable or reported; claims kept apart; the same mechanisms for itself;
the Operator keeps control; aligned evolution; the root system; generalise
the mechanism), each naming the requirements that make it concrete (for
example the Operator keeps control: `OneChangePath`, `ChildWorkBounded`,
`OnlyGivenCredentials`), "partly" or "none yet". `satisfy purpose by
agentique` is declared: a dogfood test asserts the purpose stands "only
declared" and is never calculated to hold. Dogfood: 5 passed.

**W13.8 The start of an objective** (`stage13/start`; the Operator's
amendment of C-54, made when W13.7 could not start: the start form's card
in the Conversation could not be scrolled, so Start was out of reach, and
the Operator found the form asked for too much). The Operator writes only
the intent. It is read once typing stops: Jev answers three typed questions
in one request (explore first or not; one, two, three or five improvements;
merge and adopt, merge only, or keep each reviewed change on its branch,
pushing and merging nothing, so the cycle ends there), and a set below the
confidence threshold escalates to the `escalation` role's model; when
neither answers, the defaults (one improvement without exploring, merged
and adopted), with why; without the decisions role it escalates directly,
and without the escalation role an unsure Jev gives the defaults. The same
intent is read once (a reading that fell back to the defaults is read again);
a reading replaced by further typing is told to stop, and a model call
already under way is cancelled at once, but what it had used is paid and
counted nowhere: only the reading used is the objective's cost, and a
reading is used once. The form shows what it was read as and who read it,
with switches for exploring, merging and adopting that the Operator may
change, and Start waits until the intent is read. Spend and time are
unlimited unless set (a child objective's are);
attempts (4), exploration steps (20 per run) and model calls per role keep
their defaults, and the form no longer lists each role's model (the thread
does as the objective starts). The thread's first entries say what the
intent was read as, by whom, what the Operator changed, and that no spend
or time limit is set; the reading's cost is the objective's, under the
decisions role. The card's scrolling element now carries its own height
limit. The Assistant's `propose_objective` proposes an intent only. Read
live before building: an exploring intent with one fix (Jev, 0.95, 0.4 s),
a stated change (Jev, 0.95), and "keep improving … don't merge anything"
(Jev unsure of the count at 0.36, escalated: explores, five improvements,
merging nothing, 6.8 s); and the proof's intent asking for a second fix in
the adopted build (Jev, 0.98: explores, two improvements, adopts).
Persistence as the Operator decided in §7.6 (a build before this change
cannot read an unlimited record: its list leaves it out and its check after
adoption refuses; tested for the old reader's parse). An independent review
found the "pull requests only" choice promised a pull request the
Orchestrator does not open, the persistence record incomplete, readings
repeated and never stopped, and stale governing text; corrected. When the Operator
opened it, Start and Cancel were drawn off the right edge of the
Conversation panel: the sentence about limits shared their row and did
not wrap. It has a line of its own, and the journey now checks that Start
is on screen inside the window (that check fails on the first version).

**W13.7 repair: a failure of the repository's checks attributed** (#129).
The Orchestrator reads each failed check's log into its failing tests and
their crates (`blockers.rs`), and places them against the crates the change
touches or affects. The rules:

- **The change's own failure** goes back to the implementer, as before. A
  failing test that reads files outside its crate, a failing step that is
  not a test, or a log that cannot be read counts as the change's.
- **A failure elsewhere** stops the cycle and keeps the reviewed change on
  its pull request. Where the objective may merge, a repair cycle proposes
  only the repair; it is not an improvement and does not delegate. Its
  implementer and reviewer are told its scope. Once it is merged, the
  blocked change is carried onto it, but only if the repair touches what
  failed and the change's patch is unchanged. It is merged only when the
  repository's checks pass on it, and that build is tried and adopted.
- **A check that never reached a verdict** is the machinery's: the change
  waits, and nothing reruns it.

Tested:
- #125's actual log, attributed elsewhere for a Studio-only change.
- A test reading outside its crate, a dependency's change, the model, a
  lint and a crashed target, each attributed to the change.
- Setup failures and cancellations, attributed to the machinery.
- Several checks combined into one attribution.
- A repair that does not touch what failed, refused for carrying.
- The record's repair rules, and the commit a cycle builds.
- A reviewed patch carried onto a moved base, and one the base changed
  under it refused.

An independent review found the trial of a carried change checking the
wrong commit, carries lost on a stop, and a repair that touched nothing
able to unblock by rerunning; corrected.

Not tested end to end: the repair → carry → build → try path in `run.rs`,
and the host's side (pushing, the pull request's checks, merging), have no
stand-in in the tests.

**W13.7 repairs: exploration** (the Operator's live proof explored the
Studio and failed: the lead's plan named Agentique's own model, but the
Driver alternated its start between `models/url-shortener` and `model`,
so two of four explorations opened the URL shortener and found nothing
relevant; decisions took p50 49 s, p95 120 s, and a 20-step exploration
showed nothing for 11 to 16 minutes; the only finding was a wrong
expectation, and nothing tested the engineering question).

*E1, what an exploration explores* (#130, `stage13/explore-target`; the
Orchestrator's lock confirmed for its docs on the Operator's instruction for
these repairs). The lead's `submit_exploration` names its `project` (a
folder of the repository that holds a model's `.sysml` files at the base
commit; the brief lists them, `model` being Agentique's own, leaving out the
pinned standards and what the code holds, `crates/`), the goal and,
where useful, `scope` (elements of that project's model, resolved at the
commit through a fresh repository holding a copy of its files), `start` (a
view's command and an element to select, carried out by rule as the first
steps after each start, so replays take them too) and `vary` (other
projects, only when the objective asks for several); a project that holds no
model, a name that does not resolve or a view that is not a command's id is
refused with the reason and the projects there are. A plan the lead's turn
accepts is recorded on the objective at once (`target`, optional in
`objective.json`), so a child delegated after it in the same turn explores
it; before the first plan, `delegate` must name the child's `project`, and a
child explores only the project it was given (its lead's plan of another is
refused). Later plans keep to the plan's projects; a lead that plans nothing
is asked once more and the cycle then ends: the alternation between
`models/url-shortener` and `model` is gone. Later explorations (also after an
adoption), the replays of fixed and known findings (only those of its
projects) and the evaluation's exploration of the changed areas use its
project at the build's commit. Each exploration and each replay checks the
copy its instance opened (copied from the project's folder of the build's
commit, the same digest of its model files, the project the observation
shows; the project's git tree is recorded beside it) before acting: a
mismatch acts on nothing, fails the exploration's cycle with what was
planned and what was opened in the thread, diverges a replay and fails the
evaluation's exploration. The digest is of the checkout's files, which are
the commit's but for files git ignores. Tests: the stand-in harness's
regression (`every_exploration_child_and_replay_of_a_targeted_objective_opens_its_project`:
an unbound child refused before the plan, two children taking `model`, a
child's lead refused another project, two cycles, the replays, all on
`model` at the base), a lead that plans nothing, a copy of other files, the
engine's checks of a copy and of the start steps, a plan's reading, a
child's project, the projects of a commit and a project's model.
`two_explorations_without_a_new_problem_end_the_objective` now asks for
variety (`vary`) to explore a second project, as alternation needs that
permission. An independent review found a child delegated before the first
plan unbound (and the regression not proving inheritance), the alternation
kept as a fallback, a plan accepted in a turn lost when the turn delegated,
replays not checking their copies, and a mismatch in the evaluation counted
as not run; corrected. Its second review found that a copy that did not
match during a reproduction set the finding not reproduced for good: such a
replay is now no result (the finding stays as it was and the cycle ends), a
finding never replayed nor judged is reproduced when found again, a fixed or
known finding whose project is gone is not replayed (said), projects given to
children before the first plan stay among its projects, a refused re-plan is
asked again, the planner is built only where the lead may delegate, and the
projects leave out only `standards/` and `crates/`; corrected, with a test of
a mismatch during reproduction only. Not changed: observation criteria and the
evaluator's judgment instances still open the change's checkout (the
repository's own model) as the project. Not verified: a live run on a real
test instance (the window checks are the coordinator's).
