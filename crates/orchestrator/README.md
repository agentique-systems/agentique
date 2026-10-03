# agq-orchestrator

The Orchestrator (ROADMAP §4.16, C-53; part `Orchestrator` in
`model/Agentique.sysml`): takes an objective the Operator gives in the
Studio's Objectives panel through cycles that improve Agentique itself.
Deterministic code decides each phase from recorded results; agents decide
what to change.

- **Record** (`record`): `objectives/<id>/objective.json` beside the session
  file (format 1, written atomically: intent, budgets, permissions, cycles,
  session ids, results, spend, continuation) and `journal.jsonl` (each side
  effect before and after, under a key, so resuming never repeats one).
- **A cycle** (`run`): Propose (the lead submits one improvement with
  acceptance criteria, then frozen), Implement (the implementer in the
  cycle's worktree), Check (the required checks and command criteria on a
  clean checkout of the commit, and the gates), Evaluate (a debug build of
  the commit as a test instance, operated through its control interface:
  observation criteria asserted, judgment criteria by the evaluator),
  Review (a fresh reviewer session), Repair (bounded; the same failures
  twice, or no fewer in two rounds, stop the cycle), Merge (push, pull
  request, the repository's checks, a squash merge of exactly the reviewed
  commit, the local default branch fast-forwarded), Build (release, with
  its manifest), Try (the build as a test instance), Adopt (the
  continuation saved, the Studio hands over to the launcher) and Resume (in
  the adopted build).
- **Gates** (`gates`): protected paths and agent configuration, configured
  keys, locked elements, and the baseline guard on tests and checks; a
  check that did not run is a failure.
- **Roles** (`roles`): lead, implementer, reviewer and evaluator, each its
  own Claude Agent runtime session with its instructions, tools and
  permission policy, and one tool to hand its result over.
- **Forge** (`forge`): `git` and `gh` as exact commands through Execution;
  never a force-push, never a push to the default branch.
- **Builds** (`builds`) and **test instances** (`control`).

Budgets (spend in USD at the models' prices, improvements, attempts per
cycle, hours worked) and the Operator's Pause, Step, Resume, Stop and
messages apply while an agent works. Every checkout, worktree, test
instance and the shared build folder live in the local app data's `work`
folder, whose `.cargo/config.toml` keeps builds without debug information.
