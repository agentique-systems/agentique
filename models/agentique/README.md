# Agentique architecture model

`Agentique.sysml` describes the logical architecture of Agentique in SysML v2
textual notation. It is the contract for our own code: when the architecture
changes, update the model in the same change.

- Each major part is a `part def` with a one- or two-sentence `doc`. The crates
  that implement it are part usages typed by `Crate` and named after the Cargo
  package, for example `part 'agq-kernel' : Crate;`.
- Allowed dependencies between parts use the standard SysML `dependency`
  relationship, from client to supplier: `dependency from Studio to SystemState;`.
- A crate may depend on crates of its own part and of every part reachable
  through these dependencies.
- A dependency that exists only because of code due for retirement is marked
  temporary in its `doc`, which names the stage that removes it.
- Simulation (Stage 4) and ImplementationLinks (Stage 5) are planned parts
  without crates.

## Check

`tools/check_architecture.py` compares the model with `cargo metadata` and prints
one line per problem when:

- a workspace crate is not mapped to any part, or a mapped crate is not in the
  workspace;
- a crate's normal or build dependency on another workspace crate is not allowed
  by the model;
- a LanguageCore or SystemState crate depends on a UI, network or AI library (the
  list is in the script).

Dev-dependencies are ignored, so tests may use any crate. `crates/studio-native`
is read as well while it keeps its own Cargo workspace. The check understands
only the syntax this model uses and rejects anything else.

Run from the repository root:

```sh
python3 tools/check_architecture.py
python3 -m unittest discover -s tools -p test_check_architecture.py
```
