# Agentique

Agentique is a native desktop application in which a person (the Operator)
and AI agents design, simulate and implement systems together, working at the
level of system architecture rather than code.

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

Stage 4 (live proof and foundations, ROADMAP §6.2) is built, provisionally
complete and pending the Operator's acceptance; Stages 0–3 wait for the
Operator's live run of Scenario A (C-29). The Assistant runs through the
provider layer on rig: with only `DEEPSEEK_API_KEY` set it uses
`deepseek-flash` (C-35); Anthropic, OpenAI and OpenRouter keys also work (only
DeepSeek has been tried live). See [docs/stages.md](docs/stages.md) for what
works, what was measured and what to try. The language core can also be tried
on its own:

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

## Checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python tools/check_architecture.py
```

`tools/check_architecture.py` compares the crate dependency graph with the
architecture model in `models/agentique/` (R-15). CI runs all four on every
pull request.

## Repository layout

| Path | Purpose |
|---|---|
| `ROADMAP.md` | Governing direction, decisions and stage plan |
| `AGENTS.md` | Short working rules for AI agents |
| `docs/stages.md` | Progress record, one section per stage |
| `models/agentique/` | Agentique's own architecture in SysML, checked against the crates |
| `crates/language` | The language core: the SysML subset as an element tree (parse, print, validate) |
| `crates/system-state` | The System State: typed operations, locks, undo, change events; `Project` ties it to History |
| `crates/history` | The model folder in git: crash-safe saves, checkpoints, branches |
| `crates/assistant` | The Assistant: tools over the System State, the turn loop, skills, the conversation, the evaluation set |
| `crates/providers` | Providers: model providers through rig, capabilities, keys, usage |
| `crates/studio-native`, `crates/studio-scene` | The Studio application (Surface, Panels, Conversation) and its Surface layout and rendering |
| `models/url-shortener/` | The Scenario A architecture, used by tests and the language check |
| `docs/` | `stages.md` (progress), `subset.md` (supported SysML), `deviations.md` (departures from the standard) |
| `standards/` | Pinned KerML 1.0 / SysML 2.0 artifacts, libraries and grammar, kept as the reference (never edited) |
| `tools/` | The architecture check and the (rarely run) standards pinning tools |
| `KerML.pdf`, `SysML.pdf`, `SysAPI.pdf` | The pinned OMG specifications |
| `Agentique-Specification-v0.1.html` | The original specification, kept as history |
