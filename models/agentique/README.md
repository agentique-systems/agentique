# Agentique architecture model

`Agentique.sysml` describes the logical architecture of Agentique in SysML v2
textual notation (REALIGNMENT §3.6). It is the contract for our own code: when
the architecture changes, update the model first, in the same change.

- Each major part is a `part def` with a one- or two-sentence `doc`. The crates
  that implement it are part usages typed by `Crate` and named after the Cargo
  package, for example `part 'agq-language' : Crate;`.
- Allowed dependencies between parts use the standard SysML `dependency`
  relationship, from client to supplier: `dependency from Studio to SystemState;`.
  `dependency` is the standard concept for "requires"; REALIGNMENT §3.6 calls
  these connections.
- Every allowed dependency is listed. A crate may depend on crates of its own
  part and of the parts its part depends on directly; dependencies are not
  transitive, so a shortcut around the System State shows up in the model.
- A dependency that exists only because of code due for retirement is marked
  temporary in its `doc`.
- Simulation (Stage 4) and ImplementationLinks (Stage 5) are planned parts
  without crates.

## Check

`tools/check_architecture.py` compares the model with `cargo metadata` and prints
one line per problem when:

- a workspace crate is not mapped to any part, or a mapped crate is not in the
  workspace;
- a crate's normal or build dependency on another workspace crate is not allowed
  by the model;
- LanguageCore depends on another part;
- a LanguageCore, SystemState or History crate depends on a UI or network library
  (the list is in the script).

Dev-dependencies are ignored, so tests may use any crate. The check understands
only the syntax this model uses and rejects anything else. CI runs it on every
pull request.

Run from the repository root:

```sh
python tools/check_architecture.py
python -m unittest discover -s tools -p test_check_architecture.py
```
