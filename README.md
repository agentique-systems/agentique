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

The direction, decisions and stage plan are in [REALIGNMENT.md](REALIGNMENT.md).
Progress per stage is in [docs/stages.md](docs/stages.md).

## Current stage

The realignment is in progress. Stage 0 (archive and clean) and Stage 1 (the
language core, `crates/language`) are done, pending the Operator's acceptance;
the Studio foundation (Stage 2) and the Assistant (Stage 3) follow. Try the
language core on the Scenario A model:

```text
cargo run -p agq-language --example check -- models/url-shortener
``` See [docs/stages.md](docs/stages.md) for what works today and
what to try.

Everything retired by the realignment is preserved at the git tag
`archive/pre-realignment`.

## Build and run

Requirements: Windows 10 or later (Linux builds in CI), the Rust toolchain
pinned in `rust-toolchain.toml`, and a C/C++ build toolchain for the bundled
SQLite.

Run the Studio:

```text
cargo run --release -p agq-studio-native
```

Until Stage 2 moves the Studio onto the new System State, it still uses the
pre-realignment language engine, which needs the precomputed standard library
bundle installed under `~/.agentique` (the Studio's setup screen installs it
from a `.agq-runtime` file). To look at the interface without it, open the
labelled visual fixture:

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
| `REALIGNMENT.md` | Governing direction, decisions and stage plan |
| `AGENTS.md` | Short working rules for AI agents |
| `docs/stages.md` | Progress record, one section per stage |
| `models/agentique/` | Agentique's own architecture in SysML, checked against the crates |
| `crates/` | The Rust crates; each part of the architecture maps to crates there |
| `adapters/modeling-sqlite/` | The SQLite revision store, replaced by git in Stage 2 |
| `standards/` | Pinned KerML 1.0 / SysML 2.0 artifacts, libraries, grammar and generated descriptors (never edited by hand) |
| `tools/` | Metamodel and grammar generators, standards pinning tools, the architecture check |
| `KerML.pdf`, `SysML.pdf`, `SysAPI.pdf` | The pinned OMG specifications |
| `Agentique-Specification-v0.1.html` | The original specification, kept as history |
