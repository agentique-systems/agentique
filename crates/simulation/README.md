# agq-simulation

Simulation (ROADMAP §4.14; part `Simulation` in
`models/agentique/Agentique.sysml`): scenarios run over a fixed snapshot of
the model, with results that say what actually ran. It depends on the
language core only: it never changes the System State and never reaches a
provider, a process or the network by itself.

```rust
let program = compile(&tree, scenario)?;                    // Err: what keeps it from running
let digest = digest::model_digest(&tree, scenario);          // provenance of the model slice
let result = run(&program, digest, &Request::new(Mode::Model), Answers::StandIns, cancel);
assert!(freshness(&result, &tree).is_current());
store.save(&result)?;                                        // app data, never committed
```

- **A scenario is a `verification def`**: `subject`, `objective { verify r; }`,
  stand-ins (`Scenarios::StandIn` usages), steps (`send … via`, `accept … via`,
  `accept after`) and checks (`assert constraint`), in order.
- **`compile`** copies what a run needs into disposable structures: the steps,
  the stand-ins with their outputs, the types (generals, item fields, enum
  values) and, for model execution, the instances of the subject's parts with
  attribute slots, ports, routes (connections, delegations, an agent's
  fallback) and state machines. Problems the language reports at elements the
  scenario depends on, and constructs the runner cannot run, are `Blocker`s
  at those elements; problems elsewhere do not block.
- **`script`** runs the steps against a `Target` and evaluates the checks. The
  model engine is one target; the implementation runner (in
  `agq-implementation`) is another, so a scenario means the same wherever it
  runs. A check that reads internal state is `unsupported` for a target that
  cannot see it. After the last step every pending event runs, and anything
  the subject sent that the scenario did not accept fails the implicit check
  "no unexpected output".
- **`engine`** is model execution: logical milliseconds; events in order of
  time, then scheduling; run to completion; `ambiguous-transition`,
  `unhandled-message`, `missing-behaviour`, `missing-connection` and the other
  stop reasons of `result::StopReason`; limits on events, logical time,
  completion depth and wall-clock time; cancellation.
- **Agents** answer from the scenario's stand-ins (`model`), from recordings
  matched by the SHA-256 of the canonical request (`replay`, never falling
  through to a live call), or from a `LiveModel` the Studio hands to an
  explicit live evaluation (`live`, several samples, per-check counts with a
  95% Wilson interval, failure categories, cost and provenance). The contract
  is checked the same way for each: the answer's type and values, a
  confidence between 0 and 1, `minConfidence` and `maxLatencyMs`; a failure
  goes to the fallback, with the reason in the trace. A live sample may take
  minutes (a model run, ten seconds); a live result keeps its answers, to be
  kept as recordings.
- **`result`** keeps the five claims apart (valid, executable, completed,
  check passed, implementation agrees) and the six verdicts;
  `RunResult::describe` says it in plain words (for the Assistant);
  **`digest`** and **`freshness`** work out whether a result still describes
  the model.
- **`runner::BackgroundRun`** runs on its own thread; the Studio polls it.

Tests: `model.rs` (the retrying dispatcher of `models/notifications`),
`agents.rs` (the link screening of `models/link-screening`), `store.rs`.
