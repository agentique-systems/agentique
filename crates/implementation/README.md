# agq-implementation

Implementation (ROADMAP §4.15; part `Implementation` in
`models/agentique/Agentique.sysml`): what the model says about code. It
depends on the language core, Simulation (the run contract, the step
interpreter and the check evaluator) and Execution (processes). It never
writes code.

- **Links** (`links`): `model/links.json`, saved with the model by History.
  Elements to modules, symbols, types, tests, entry points, schemas,
  configuration, instructions and crates, many to many, by element identity;
  `for_path` looks up code to model. Also the harness command and the paths
  an implementation task may never change.
- **Checks** (`checks`), each with its coverage (`CheckKind::coverage`):
  - dependency boundaries: linked modules' `crate::` references against the
    model's `dependency` relationships, or (the dogfood check) a Cargo
    workspace's crates against a self-model that maps crates with usages of
    a `Crate` part def;
  - contract shapes: a linked Rust struct or enum against its item or enum
    def (field and variant names, simple types, optional fields);
  - linked tests: `cargo test` for the linked tests, one result each.

  Failing checks are drift at the elements they cover (`drift`). A file
  existing at a linked path is never counted as conformance.
- **Scenarios against the implementation** (`harness`): the runner starts
  the linked harness and drives it one JSON line per step (the protocol is
  documented in `harness.rs`); the harness calls the real code with the
  scenario's stand-ins in place of the parts they name; the runner evaluates
  the expressions and checks with Simulation's interpreter. Checks that read
  internal state are `unsupported` here.
- **`CheckReport`** keeps a round of checks with the commit, the working
  tree's digest and the model digest the links reach, so freshness can be
  computed.

Tests: `notifications.rs` runs everything against a small real Rust
repository (`tests/fixtures/notifications`, the retrying dispatcher, with its
harness), including deliberate breaks; `dogfood.rs` validates Agentique's own
self-model with its own core and checks its crates against it.
