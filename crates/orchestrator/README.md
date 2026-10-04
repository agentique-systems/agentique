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
- **A cycle** (`run`): Propose (the lead, reading but running no command,
  submits one improvement with acceptance criteria, then frozen; a command
  criterion is a test run: `cargo test`, `node --test` or
  `python -m unittest`, nothing else), Implement (the implementer in the
  cycle's worktree, which starts at the cycle's base), Check (the command
  criteria first on the base, where they must fail; then the required checks
  and the criteria on a clean checkout of the commit, where each criterion
  must pass and run at least one test; and the gates), Evaluate (a debug build of
  the commit as a test instance, operated through its control interface:
  observation criteria asserted, judgment criteria by the evaluator),
  Review (a fresh reviewer session that runs no command), Repair (bounded;
  the same failures twice, or no fewer in two rounds, stop the cycle), Merge
  (one commit holding the reviewed tree, on the last pushed or the base, is
  pushed, so no earlier attempt leaves this computer; the pull request, the
  repository's checks, a squash merge of exactly that commit, the local
  default branch fast-forwarded and an open project read again), Build (release, with
  its manifest), Try (the build as a test instance), Adopt (the
  continuation saved, the Studio hands over to the launcher) and Resume (in
  the adopted build).
- **Gates** (`gates`): protected paths, agent configuration at any depth
  and Agentique's safeguards (unless the objective names them), configured
  keys (in the change, the pushed commit's message and the pull request),
  locked model elements, the code of locked parts (through the
  implementation links), the criteria failing before the change, and the
  baseline guard on tests and checks (each weakened assertion, threshold or
  test, line by line); a check that did not run is a failure.
- **Roles** (`roles`): lead, implementer, reviewer and evaluator, each its
  own Claude Agent runtime session with its instructions, tools and
  permission policy, and one tool to hand its result over.
- **Models per role** (`models`, C-54): each role's model (the four above,
  the explorer, escalation and typed decisions), resolved by the Studio
  from Settings › Agents and the credentials before an objective starts: its
  own model when its provider has a credential Agentique may use for that
  kind of role (a session may use an API key or the Claude subscription
  token; a direct call needs a key), otherwise its fallback with the reason,
  otherwise the objective does not start. The record keeps each role's
  model, effort, credential kind and source, who pays and any fallback's
  reason; every session of a role is built on it. An objective that does
  not explore needs only the lead, implementer, reviewer and evaluator; the
  others are recorded as without a model. `decider` builds typed decisions
  from the `decisions` and `escalation` roles (W12.5 uses it).
- **Forge** (`forge`): `git` and `gh` as exact commands through Execution;
  never a force-push, never a push to the default branch.
- **Builds** (`builds`) and **test instances** (`control`).
- **Exploration** (`explore`, `findings`, `knowledge`; C-54): an explorer
  operates a test instance toward a goal (behind the `Instance` boundary:
  a Studio started fresh from a copy of the start project, inside its own
  folder, or a stand-in in tests). Each step lists the actions valid there
  that the observation offers to agents (fields with fixed input classes),
  chooses one by the rules, Jev, the explorer's model or Jev escalating,
  acts, and checks the invariants. A finding is a check that failed; it is
  reproduced by two replays from a fresh start (no model asked) and
  reduced; `replay` is what a cycle's criterion runs. The testing
  knowledge (`testing/<project>/knowledge.json`, format 1, beside the
  objectives' records) keeps coverage, findings and runs across runs.

Budgets (spend in USD at the models' prices, usage without a price at a
high one, kept by role and by model within it; improvements, attempts per
cycle, hours worked) and the Operator's
Pause, Step, Resume, Stop and messages apply while an agent works. Commands
an implementer runs are not confined (they run with the Operator's rights,
as any build does); the gates check what reaches the change, and GitHub's
branch protection on the default branch is the backstop for anything else. Every checkout, worktree, test
instance and the shared build folder live in the local app data's `work`
folder, whose `.cargo/config.toml` keeps builds without debug information.
