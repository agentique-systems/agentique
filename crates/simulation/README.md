# agq-simulation

Simulation (ROADMAP §4.14; part `Simulation` in
`model/Agentique.sysml`): scenarios run over a fixed snapshot of
the model, with results that say what actually ran. It depends on the
language core only: it never changes the System State and never reaches a
provider, a process or the network by itself.

```rust
let program = compile(&tree, scenario)?;                    // Err: what keeps it from running
let digest = digest::model_digest(&tree, scenario);          // provenance of the model slice
let result = run(&program, digest, &Request::new(Mode::Model), Answers::StandIns, cancel);
assert!(freshness(&result, &Present::model(&tree)).is_current());
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
- **Execution identity** (C-52). For a replay or a live evaluation the run's
  `Request` carries a `RunBinding` the Studio prepared: the one agent
  configuration it covers (a call from another stops the run as
  `unsupported`) and its `Binding` (provider, model, the provider layer's
  adapter and the Studio's mapping with their revisions, the exact question,
  what of the input is sent, the adapter's policy; strings and JSON only).
  The binding goes into every `AgentRequest` and so into the recording key;
  without one, a request has exactly the canonical bytes of the time before
  bindings, so old recordings still replay unbound requests but never answer
  a bound one (the stop says why). `Recordings::read` checks each key again.
  `describe_agents` lists a compiled scenario's agents with their effective
  settings, answer fields (the confidence found by identity) and inputs.
- **Live calls** (C-52) get `CallLimits`: one deadline, the nearer of the
  agent's `maxLatencyMs` and the run's wall-clock limit. A provider failure
  is kept as a call (`providerError`, its time and unknown cost counted) and
  ends the evaluation; a stop during a call is a stop, whatever the client
  reported; `Limits::max_live_calls` is an allowance shared by every sample,
  and the call beyond it is not made (`budget-exhausted`). A call of
  unknown cost is counted (`LiveSummary::unknown_cost`): the total is then
  absent and `known_cost_usd` holds the known part. A recording keeps the
  provider's `Evidence` (the model that answered, its raw estimates, usage,
  request id, attempts), apart from the agent's output.
- **`result`** keeps the five claims apart (valid, executable, completed,
  check passed, implementation agrees) and the six verdicts;
  `RunResult::describe` says it in plain words (for the Assistant);
  **`digest`** and **`freshness`** work out whether a result still describes
  the present (`Present`): the model slice and the runner, and for a replay
  or a live result also the binding and, for a replay, the recordings. One
  without a binding, made before bindings were recorded, is never current.
- **`runner::BackgroundRun`** runs on its own thread; the Studio polls it.

Tests: `model.rs` (the retrying dispatcher of `models/notifications`),
`agents.rs` (the link screening of `models/link-screening`), `identity.rs`
(bindings, keys, legacy recordings, deadlines, accounting, freshness, what
an older build reads), `store.rs`.
