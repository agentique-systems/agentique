# Independent authored Vehicle construction

The rich Vehicle fixture now has an independently authored kernel `ChangeSet`
counterpart. The builder creates definitions, usages, owned memberships, typing,
subclassification, redefinition, connection ends/reference subsettings, state
subaction roles, and a requirement subject/constraint expression. It does not
clone lowered records or consume the textual graph while constructing its model.

The comparison covers all named declaration metaclasses and direct Usage types,
inherited members of SportsCar/TurboEngine/MeteredFuelPort, behavior and
requirement member populations, original inherited identities, redefinition
suppression, and ordered connection endpoints. Each named declaration occurs
once per graph. Textual and programmatic element IDs differ; query result
comparison normalizes names, retaining explicit original-ID assertions within
each graph. Programmatic records retain authored provenance without syntax.

Both graphs are strictly valid kernel snapshots. Queries explicitly carry no
producer-closure certificate. These are **current graph** checks, not accepted
Systems integration, effective SysML completion, execution, or language readiness.
Phase G remains gated on actual Systems publication acceptance.

Low-artifact verification uses the isolated `target/foundation-evidence` with
incremental compilation and dev/test debug information disabled, two build jobs.

- `cargo test --locked --offline -p agq-kerml-text --lib rich_vehicle -- --nocapture`:
  2 tests passed, 0 failed; exit 0.
- `cargo clippy --locked --offline -p agq-kerml-text --all-targets -- -D warnings`:
  exit 0, no warnings.
- `cargo fmt --all -- --check` and `git diff --check`: exit 0.

No whole-library publication candidate or workspace test was run for this test
preparation.
