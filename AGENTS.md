# Agentique: instructions for agents

Agentique is a native desktop application in which a person (the Operator)
and an AI Assistant design, simulate and implement systems together, working
at the level of system architecture. `REALIGNMENT.md` is the governing
direction. This file is its short working summary; where they differ,
`REALIGNMENT.md` wins.

## Rules

1. **`REALIGNMENT.md` governs.** Every piece of work names the stage (§5) and
   the scenario step, decision (C-n) or recommendation (R-n) it serves. If it
   serves none, do not start it; propose an update to `REALIGNMENT.md` instead.
2. **Work only in the current stage** (see `docs/stages.md`). Items that wait
   do not start early, however attractive.
3. **Read REALIGNMENT §1.3 (slop) before designing anything.** Prefer the
   simplest design. Generalise instead of multiplying parts. Use standard
   names. Understand the intent before acting on the words, and ask when the
   intent is unclear.
4. **The stable core is locked** (R-16). Changing the language core, the System
   State operations or the persistence format requires an explicit Operator
   decision, recorded in REALIGNMENT §6.
5. **Changes that cut across parts start in the self-model.** Update
   `models/agentique/` first. The dependency check (R-15) must stay green.
6. **No new crate, top-level folder, register, generator or evidence format**
   without a distinct responsibility in the self-model and a reason the
   Operator can see.
7. **Standards are a reference, not a project.** Consult the pinned KerML 1.0
   and SysML 2.0 specifications to get concepts right. Extend the subset only
   for a scenario need and record it in the subset manifest. Record any
   deviation from the standard in one line with its reason.
8. **Preserve the pinned standards.** Never modify `standards/artifacts/`, the
   library archives and sources under `standards/`, the PDFs or
   `Agentique-Specification-v0.1.html`. Acquiring standards artifacts is never
   a build step.

## Checks

Run these before handing work back, and report the actual results honestly
("works", "partially works", "not tried" and "failed" are different
statements):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python tools/check_architecture.py
```

A green test suite does not mean a stage is done: a stage is complete when the
Operator has used its outcome and accepts it. Do not commit generated logs,
receipts, screenshot hashes or other machine-generated evidence.

## Naming

- Model concepts use KerML/SysML names: part, port, interface, connection,
  item, attribute, requirement, state, action, specialisation, redefinition.
- Everything else uses plain software terms: change, branch, commit, view,
  validation error, undo. Do not use "candidate", "World", "publication",
  "receipt", "frontier", "authority", "fabric" or "rematerialization".
- Product nouns: Studio, Surface, Panels, Conversation, Assistant, System
  State, lock, scenario, simulation, implementation link, drift.

## One truth per topic

| Topic | Document |
|---|---|
| Direction and decisions | `REALIGNMENT.md` |
| Stage progress | `docs/stages.md` |
| What Agentique is and how to run it | `README.md` |
| Architecture | `models/agentique/` |
| Supported KerML/SysML constructs and deviations | the subset manifest and deviations list (Stage 1) |
| A crate's purpose | its own short `README.md` |

Edit the governing text in place. Never keep versioned copies (`-v2`,
`phase3`); git keeps the history. Retired work is at the tag
`archive/pre-realignment`.
