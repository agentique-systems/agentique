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
