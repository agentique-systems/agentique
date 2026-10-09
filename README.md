# Agentique

Agentique is a system for understanding, modelling, simulating, verifying,
implementing and evolving systems through explicit KerML/SysML v2
architecture, and it is itself such a system; its purpose is stated once, in
[ROADMAP.md §1.1](ROADMAP.md). It is a native desktop application in which a
person (the Operator) and AI agents design, simulate and implement systems
together, working at the level of system architecture rather than code.

The Operator works in the **Studio**, which has two equal ways to work:

- the **Surface**, a visual, spatial view of the system with side panels, where
  the architecture is explored and changed directly;
- the **Conversation**, a chat with an AI **Assistant** that changes the same
  architecture through typed tools.

Both change one **System State**, built from KerML and SysML concepts (parts,
ports, interfaces, connections, items, attributes, requirements). SysML text is
only the storage format; the Operator never has to read or write it.

The direction, decisions and stage plan are in [ROADMAP.md](ROADMAP.md).
Progress per stage is in [docs/stages.md](docs/stages.md).

## Current stage

Stage 13 (Agentique models, simulates and evolves systems, itself included,
C-55, ROADMAP §6.10) is in progress: referential usages and requirement
constraints with their evaluation, the self-model's purpose and autonomous
lifecycle, a second example system, and autonomous cycles that name the
requirement they serve and are reviewed against the Operator's approved
baseline. Stages 11 and 12 (Agentique improves, then tests and improves,
itself) are built and wait for the Operator's own run and acceptance; see
[docs/stages.md](docs/stages.md) and, below, "Agentique improves itself".

Stages 7–8 (the factory loop, C-50, ROADMAP §6.5) are in progress: agents
in the model, scenarios that run against the model, recordings, a live model
or the real code, implementation links and checks, and a supervised
implementation loop in which the Assistant writes code in a worktree for the
Operator to review. Built and tested; nothing is accepted yet. To try it,
start from the URL shortener "And its code" (below). Stage 5 (the daily-use
Studio and Settings, ROADMAP §6.3) is also still in progress:
the Studio on GPUI with a redesigned presentation (C-48), the design tokens
and the component gallery, Settings (Ctrl+,)
with keys in the Windows Credential Manager, conversation format 2,
incremental Surface updates (10k pan and zoom within budget), cost per turn
and per day, the first run's welcome, focus mode and collapsible panels, and
the standard shortcuts, a minimap and a model picker are in; the design
system, the Surface drawing and the Conversation's tool cards are redrawn on
GPUI and wait for the Operator's use. Stage 4
is provisionally complete and Stages 0–3 wait for the Operator's live run of
Scenario A (C-29). With only a DeepSeek key (saved in Settings, or
`DEEPSEEK_API_KEY`) the Assistant uses `deepseek-flash` (C-35). See
[docs/stages.md](docs/stages.md) for what works, what was measured and what to
try. The language core can also be tried on its own:

```text
cargo run -p agq-language --example check -- models/url-shortener
```

Everything retired by the realignment is preserved at the git tag
`archive/pre-realignment`; `REALIGNMENT.md`, which governed Stages 0–3, is
retired to git history (last version on `main` at `6fc90b78`).

## Build and run

Requirements: Windows 10 or later (Linux builds in CI and needs OpenSSL's
development files for TLS), the Rust toolchain pinned in
`rust-toolchain.toml`, and a C/C++ build toolchain for the embedded git library
(libgit2). No git install or runtime bundle is needed; the network is used only
by the Assistant, with a provider key.

Run the Studio:

```text
cargo run --release -p agq-studio-native
```

The Studio opens a start screen: create a new project (a folder; its model is
kept in `model/` inside it, in git) or open an existing one. Try the Scenario A
model by opening a project and building the URL shortener by hand, or look at
the labelled visual fixture:

```text
cargo run --release -p agq-studio-native -- --fixture architecture --no-restore
```

The factory loop (Scenario I, ROADMAP §2.10): on the start screen choose
"Start from the URL shortener", then "With AI screening" or "And its code"
(the second also writes its Rust code beside the project, committed and
linked). Then:

- Ctrl+Shift+R shows the scenarios; choose one and press F5 to run it in the
  chosen mode (model, replay, code, live, walkthrough). The Run panel keeps
  the result, its checks and their reasons, and a trace to step through
  (`[`, `]`, `\`) that the Surface follows. A result that no longer
  describes the model says so and is not drawn.
- The Run panel writes scenarios without SysML: add what goes in, what to
  wait for, stand-ins for the parts they replace, checks and time.
- Code runs only after "Trusted-local execution" is allowed for the project
  (the Run panel asks). The Inspector's Implementation section shows the
  linked code and drift, checks the implementation, and starts "Implement
  with the Assistant…": a worker writes code in a worktree, the Studio checks
  it, and you review the patch before integrating it.
- A live evaluation costs money and always asks first: the confirmation
  shows the model and how it is asked, at most how many calls, what leaves
  the computer and a cost bound, and the plan is fixed once confirmed.
  `TypedScreeningCases` evaluates the screening agent with a typed decision
  model (TypeSafe AI's Jev, `TypedLinkScreening`) beside the chat agent;
  a model the provider layer does not know is refused, never replaced.
  Replay answers only from recordings made with the same provider, model
  and mapping.

Agentique improves itself (Scenario J, ROADMAP §2.11). Once, to bootstrap:
save a DeepSeek or Anthropic key in Settings › Providers; choose the Claude
Agent runtime in Settings › Assistant and install it; start the Studio on
this repository (`cargo run --release -p agq-studio-native -- --project <this repository>`),
allow "Trusted-local execution…" (Ctrl+K), then in Settings › About › Builds
choose Build and, when it is built, "Use this build…". That installs the
launcher in `%LOCALAPPDATA%\Agentique\builds`. From then on start Agentique
supervised, for example from a shortcut:

```text
%LOCALAPPDATA%\Agentique\builds\agentique-launcher.exe --supervise --project <this repository>
```

Then open the Objectives tab (beside Problems), say what to improve, set the
spend budget and how many improvements, leave merging and adopting on, and
press Start. The panel and the status line show each phase, the activity of
the lead, implementer, reviewer and evaluator, and the spend; Pause holds the
agents before their next tool call, Step lets one through, Stop ends the
objective, and a message reaches the agent at work. A change is merged only
when the required checks, its criteria, the gates and an independent review
pass and CI passes on the pushed commit; Agentique then builds it, tries it in
a test instance and restarts in it. Closing Agentique interrupts an objective
(Continue resumes it); a build that does not start gives way to the last
known good one, which says why.

The design tokens, the generated themes and the components are shown in one
place by the component gallery:

```text
cargo run --release -p agq-studio-native -- --fixture components --no-restore
```

## Checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python tools/check_architecture.py
```

`tools/check_architecture.py` compares the crate dependency graph with the
architecture model in `model/` (R-15). CI runs all four on every
pull request.

## Repository layout

| Path | Purpose |
|---|---|
| `ROADMAP.md` | Governing direction, decisions and stage plan |
| `AGENTS.md` | Short working rules for AI agents |
| `docs/stages.md` | Progress record, one section per stage |
| `model/` | Agentique's own architecture: the repository's project model (parts, contracts, the workflows as scenarios, links to code and tests), checked against the crates |
| `crates/language` | The language core: the SysML subset as an element tree (parse, print, validate) |
| `crates/system-state` | The System State: typed operations, locks, undo, change events; `Project` ties it to History |
| `crates/history` | The model folder in git: crash-safe saves, checkpoints, branches |
| `crates/assistant` | The Assistant: tools over the System State, the turn loop, skills, the conversation, the evaluation set, the implementation worker |
| `crates/library` | The Library of building blocks: built-in blocks (with behaviour and scenarios), the project's definitions and My Library |
| `crates/simulation` | Scenarios run: compiling, the model engine with stand-ins, recordings and live agents, results, traces and freshness |
| `crates/implementation` | Implementation links, the supported checks and drift, the harness runner, task briefs and verification |
| `crates/execution` | The executor every process goes through: scopes, trusted-local execution, git worktrees and patches, jobs |
| `crates/providers` | Providers: model providers through rig, capabilities, keys, usage |
| `crates/studio-native`, `crates/studio-scene` | The Studio application (Surface, Panels, Conversation) and its Surface layout and rendering |
| `models/url-shortener/` | The Scenario A architecture, used by tests and the language check |
| `models/link-screening/` | Scenario I: the URL shortener with AI screening and its scenarios; its code is a fixture in `crates/implementation/tests/fixtures/url-shortener` |
| `models/notifications/` | Scenario I's second example, a retrying notification dispatcher |
| `docs/` | `stages.md` (progress), `subset.md` (supported SysML), `deviations.md` (departures from the standard) |
| `standards/` | Pinned KerML 1.0 / SysML 2.0 artifacts, libraries and grammar, kept as the reference (never edited) |
| `tools/` | The architecture check, the link-screening evaluation's report (`screening_report.py`), and the (rarely run) standards pinning tools |
| `KerML.pdf`, `SysML.pdf`, `SysAPI.pdf` | The pinned OMG specifications |
| `Agentique-Specification-v0.1.html` | The original specification, kept as history |
