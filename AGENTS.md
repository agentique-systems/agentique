# Agentique: instructions for agents

Agentique is a native desktop application in which a person (the Operator)
and AI agents design, simulate and implement systems together, working at the
level of system architecture. `ROADMAP.md` is the governing direction. This
file is its short working summary (ROADMAP §8.1); where they differ,
`ROADMAP.md` wins. Since C-53, AI agents working inside Agentique (its
Assistant and the Orchestrator's lead, implementer, reviewer and evaluator)
are its primary users: these rules are theirs as much as any person's.

## Rules

1. **`ROADMAP.md` governs.** Every piece of work names the stage (§6) and the
   scenario step, decision (C-n) or recommendation (R-n) it serves. If it
   serves none, it does not start; propose an update to `ROADMAP.md` instead.
2. **Work only in the current stage** (see `docs/stages.md`). Items that wait
   do not start early, however attractive.
3. **Read ROADMAP §1.3 (slop) before designing anything.** Prefer the simplest
   design; generalise instead of multiplying parts; use standard names;
   understand intent before acting literally; ask when intent is unclear.
4. **The stable core is locked** (R-16). Changing the language core, the System
   State operations or the persistence format requires an explicit Operator
   decision, recorded in ROADMAP §7.6.
5. **Changes that cut across parts start in the self-model.** Update
   `model/` first (Agentique's own project model). The dependency check (R-15)
   must stay green.
6. **No new crate, top-level folder, register, generator or evidence format**
   without a distinct responsibility in the self-model and a reason the
   Operator can see.
7. **Standards are a reference, not a project.** Consult the pinned
   specifications to get concepts right. Extend the subset only for a scenario
   need and record it in the manifest. Record any deviation in one line with its
   reason.
8. **Preserve the pinned standards.** Never modify `standards/artifacts/`, the
   library archives and sources under `standards/`, the PDFs or
   `Agentique-Specification-v0.1.html`. Acquiring standards artifacts is never
   a build step.
9. **Run the checks** below before handing work back, and report the actual
   results honestly, including failures. Changes to the Studio or the Surface
   also run the reference budget run (ROADMAP §8.6).
10. **Inside an objective** (C-53, ROADMAP §4.16): change the model only with
    Agentique's tools (`apply_changes`, `use_library_block`, `link_code`),
    never by editing `model/*.sysml`, `model/agentique.json` or
    `model/links.json` as files; work only in your cycle's worktree (the file
    tools are held to it, and every path a change touches is checked before
    merging); leave `.claude/`, `CLAUDE.md` and `AGENTS.md` alone unless the
    objective names them; hand work to another agent only as a child
    objective through Agentique's delegation, within your objective's
    permissions and budgets (C-54); keep the acceptance criteria and required
    checks frozen at the proposal; never delete, ignore or loosen a test, check or
    budget to make it pass (a deliberate change to one is named, with its
    reason, for the reviewer); never force-push, push to the default branch
    (`main`) or merge yourself: the Orchestrator merges when the gates pass.
    Say what you did not verify.

## Checks

Report the actual results honestly ("works", "partially works", "not tried"
and "failed" are different statements):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python tools/check_architecture.py
node --experimental-strip-types --no-warnings --test claude-agent/test/*.test.ts
```

The last line is for changes to the Claude Agent companion (`claude-agent/`).

A green test suite does not mean a stage is done: a stage is complete when the
Operator has used its outcome and accepts it (ROADMAP §8.3). Do not commit
generated logs, screenshots, screenshot hashes, budget reports, evaluation
results or transcripts, recordings made for measurement, or other
machine-generated evidence. Evaluation task definitions and test fixtures are
code and are committed.

## Naming (ROADMAP §8.4)

- Model concepts use KerML/SysML names: part, port, interface, connection,
  item, attribute, requirement, state, action, specialisation, redefinition.
  **Agent** is the built-in library definition for AI-driven parts;
  "Assistant" is Agentique's own agent.
- Everything else uses plain software terms: change, branch, commit, view,
  validation error, undo, provider, skill, note, plan, compaction, evaluation.
  Do not use "candidate", "World", "publication", "receipt", "frontier",
  "authority", "fabric" or "rematerialization".
- Product nouns: Studio, Surface, Panels, Conversation, Assistant,
  Orchestrator, System State, Settings, lock, scenario, simulation,
  implementation link, drift, agent, autonomy mode, objective, cycle,
  permission policy, control interface, observation, typed decision,
  exploration, finding, testing knowledge, child objective, observer mode. A
  new product term needs a reason and a glossary entry (ROADMAP §9).

## One truth per topic (ROADMAP §8.2)

| Topic | Document |
|---|---|
| Direction, decisions and stages | `ROADMAP.md` |
| Stage progress | `docs/stages.md` |
| What Agentique is and how to run it | `README.md` |
| Architecture | `model/` (Agentique's own project model) |
| Supported KerML/SysML constructs and deviations | `docs/subset.md`, `docs/deviations.md` |
| Design tokens and components | the tokens module and the component gallery |
| Settings | the settings table in code |
| Budgets | ROADMAP §3.3, and the constants the harness asserts |
| A crate's purpose | its own short `README.md` |

Edit the governing text in place and add a one-line entry to ROADMAP §7.6.
Never keep versioned copies (`-v2`, `phase3`); git keeps the history.
`REALIGNMENT.md` is retired to git history (last version on `main` at
`6fc90b78`); retired work before it is at the tag `archive/pre-realignment`.
