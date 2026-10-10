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
  The record keeps its agents' directives (C-54: author, recipient role or
  child objective, scope, status, result and the record handed over), made
  only by the Orchestrator: the lead's proposal for the implementer, the
  reviewer's findings for repair.
- **Thread** (`thread`, C-54): `objectives/<id>/thread.jsonl`, what happened
  in order for the Conversation and the Objectives panel: the Operator's
  messages and commands, directives, results (submissions, verdicts), the
  Orchestrator's events (phases, checks, gates, pull requests, merges,
  builds, trials, adoptions, recoveries) and each agent's tool calls as
  activity (a bounded diff for a file changed, the command line for a
  command), each with its author (the Operator, Agentique, or a role and its
  model). Appended one whole line at a time under a lock file, numbered,
  read back from any point; capped per entry, and a file over 4 MB is kept
  as `thread.1.jsonl` while a new one starts.
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
- **Purpose and traceability** (`traceability`, C-55): a proposal names
  the requirements of the project's model it `serves`, its `benefit` and
  its effect on `complexity`; `serves` and `parts` are resolved by identity
  in the base commit's model when it is submitted (a name that does not
  resolve, or a `serves` that is no requirement, refuses it; a base without
  a model folder resolves nothing, and a model with no requirement yet
  records `serves` as stated, each saying why; a model that cannot be read
  stops the cycle). At review, what the commit changed (model elements by
  identity, and the parts whose linked code changed) is compared with
  `parts` ("changed but not named", "named but not changed"), and the
  cumulative change since the Operator's approved baseline (the tag
  `approved-baseline`, read with `git ls-remote` at the URL of `origin`
  recorded when the objective was created, never the local tag; without
  it, the objective's start, saying why) is counted at the root: both go to
  the reviewer, whose judgments of both are required and recorded, to the
  cycle's record and to the thread; the numbers inform, they do not
  decide. In a project whose model declares a root purpose requirement
  (`Purpose`, `purpose`; Agentique's does), the gate "purpose and
  governance unchanged" fails a change to `ROADMAP.md` or to that
  requirement and what it owns, by identity, also when it is moved or
  renamed, as the base, the objective's start and the approved baseline
  declare it, whatever the objective names; a failure goes to repair like
  any gate's. Models, links and their presence are read by the commits'
  trees through a clean checkout made for it, never through the checkout
  the checks ran in. The `origin` URL is recorded without credentials.
  Worktree sessions are refused `git tag` and changes to the remotes; every
  session (the Operator's Conversation too) is refused git aliases and
  includes given on the command line or in the environment, releases, tag
  refs and GraphQL mutations on the host, and `gh` without the network.
  These rules cannot close every way: the approved baseline's real
  protection is a tag ruleset on the host, recommended to the Operator.
- **Dispositions** (`findings`, `knowledge`; C-55): the lead judges a
  reproduced finding with `adjudicate_finding` (a defect, a wrong
  expectation, an ambiguous requirement or an unreliable reproduction),
  which the Orchestrator records in the testing knowledge, across
  objectives, and shows in the thread; a finding's id (`f1`, …) is its
  place among the cycle's findings and never changes. Only a defect is
  chosen to fix, and once a proposal fixing it is accepted it cannot be
  judged otherwise; a wrong expectation is not offered again (found again,
  it is not new); an unreliable reproduction is not offered again until it
  is found on another build than the one it was judged on; an ambiguous
  requirement is a question for the Operator, not reproduced again in
  later cycles. When every finding is
  judged other than a defect, the cycle ends with nothing to fix. The
  reviewer is told how the finding a change fixes was judged.
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
  from the `decisions` and `escalation` roles, and `with_deciding` gives
  exploration its `Deciding` from those and the `explorer` role (W12.5 uses
  them).
- **Forge** (`forge`): `git` and `gh` as exact commands through Execution;
  never a force-push, never a push to the default branch.
- **Builds** (`builds`) and **test instances** (`control`).
- **Exploration** (`explore`, `findings`, `knowledge`; C-54): an explorer
  operates a test instance toward a goal (behind the `Instance` boundary:
  a Studio started fresh from a copy of the start project, inside its own
  folder, or a stand-in in tests). Before its first action a run checks the
  copy against its plan's provenance (the folder copied, the digest of its
  model files, the project the observation shows) and ends without acting
  when they differ; a plan's start (a view's command, an element to select)
  is carried out by rule after each start. Each step lists the actions valid there
  that the observation offers to agents (fields with fixed input classes),
  chooses one by the rules, Jev, the explorer's model or Jev escalating,
  acts, and checks the invariants. A finding is a check that failed; it is
  reproduced by two replays from a fresh start (no model asked) and
  reduced; `replay` is what a cycle's criterion runs. The testing
  knowledge (`testing/<project>/knowledge.json`, format 1, beside the
  objectives' records) keeps coverage, findings and runs across runs.
- **Exploring cycles** (`run/explore.rs`, `run/evidence.rs`; C-54): when an
  objective explores, a cycle first explores and reproduces (the lead plans
  its target: the project, a folder that holds a model at the base commit,
  with the goal and where useful the elements it is about and where to
  start, checked when submitted; the objective records it, and every later
  exploration, child, replay and the evaluation's exploration keep to its
  projects; the lead may delegate an area; the explorer runs in a test
  instance of the base build, on a copy of the project at its commit, at the
  Operator's observer speed, after replaying the
  findings fixed since; at most three new findings, most severe first, are
  reproduced and reduced), then the lead proposes to fix one, whose replay
  is frozen as the criterion `replay`. Every criterion runs on the base
  before the change: one must fail there with evidence (the replay, an
  observation, or a test that compiles, runs and fails with the change's
  test files brought over) and none may pass. A change is evaluated in test
  instances whenever a criterion is behavioural or it touches the Studio's
  code by the links (the replay on the change, a short exploration by the
  rules of the areas it touched). Two explorations in a row with nothing new
  reproduced, with no known finding left untried, end the objective. The
  evidence is made with each checked commit's own test files, and only a
  criterion's own expectation or an assertion failing counts. Test
  instances start as such
  (`--test-instance`, `--control-speed`, the stand-in Assistant for an
  unreviewed build, the explorer's key only for a merged one), in a stated
  condition when a criterion asks (`recovered`, `with an objective`); the
  driver reaches builds and instances through `run::Studios` (`Live`, or a
  stand-in in tests).
- **Delegation** (`run/children.rs`; C-54): the lead's `delegate` tool,
  checked (budget left, two deep at most, one child at a time) and recorded
  as its directive; the child objective runs inside the parent's run,
  explores and reproduces, and its result goes to the lead's next turn.
  `Command::StopChild` stops one child; Stop stops them all. The Operator's
  messages go to the implementer while it works and otherwise wait for the
  lead's next turn.
- **Durability and bounds** (C-54): an objective interrupted because the
  Operator closed Agentique waits for Continue; one an adoption handed over,
  or that was running when the launcher recovered a crash, goes on by
  itself; two resumes without progress stop it (`Objective::on_start`). A
  cycle's worktrees are removed when it ends (a failed or interrupted
  cycle's `work` is kept, the three most recent), each by its name; merged
  branches are deleted here and on the host; one build at a time
  (`builds::lock`); test-instance folders are removed after use. An
  unreviewed build starts only as a test instance with the stand-in
  Assistant, or not at all.

Budgets (spend in USD at the models' prices, usage without a price at a
high one, kept by role and by model within it; improvements, attempts per
cycle, hours worked, an exploration's steps, model calls per session role)
and the Operator's
Pause, Step, Resume, Stop and messages apply while an agent works. Commands
an implementer runs are not confined (they run with the Operator's rights,
as any build does); the gates check what reaches the change, and GitHub's
branch protection on the default branch is the backstop for anything else. Every checkout, worktree, test
instance and the shared build folder live in the local app data's `work`
folder, whose `.cargo/config.toml` keeps builds without debug information.
