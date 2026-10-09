# Agentique's own model

`Agentique.sysml` is Agentique's architecture (ROADMAP §4.6), and this folder
is the model folder of Agentique's repository as an ordinary Agentique project
(C-51): "Develop Agentique" opens it in the Studio. It is the contract for our
own code: when the architecture changes, change the model first, in the same
change, through Agentique (the Surface or the Assistant) like any project.

How to read it, from the top down:

1. **Purpose and boundaries**: the package's documentation.
2. **Parts** (`part def`): each says what it is for, the information it owns,
   its contract and what it must not do. The crates that implement it are its
   parts typed by `Crate` (`part 'agq-language' : Crate;`). In the Studio, the
   Inspector's "About this part" answers what it is, why it exists, what it
   owns, what depends on it, where it is implemented and checked, and what
   changing it affects; the Assistant's `explain_element` gives the same answer.
3. **Contracts and interactions**: the item defs are what the parts exchange,
   the port defs say what goes in and out, and `Agentique` connects the parts
   as they work together.
4. **Allowed dependencies**: one `dependency from A to B;` per allowed use of
   one part by another (standard SysML `dependency`, client to supplier). They
   are not transitive, so a shortcut around the System State shows up here.
5. **Guarantees**: requirements, with the parts that satisfy them.
6. **Workflows**: the scenarios at the end walk an ordinary edit, an
   Assistant action and a development task across the parts, each with its
   failure paths. They run in model execution (the Scenarios tab, F5): they
   check this model's account of the workflow, not the code. The code is
   checked by the tests linked to each step.

Changes are made through Agentique (the Studio, the Assistant's tools, or
the `apply_changes` example for an agent without a window), which prints the
model as it saves it: new members are added after the existing ones, and a
member without a name (a connection, a `satisfy`) is identified by its
position among its owner's unnamed members, so they are not reordered by hand.

Beside it, as in every project:

- `agentique.json`: the identity file (element ids, and the locks: the locked
  core and Agentique's safeguards).
- `links.json`: the implementation links: each part to its crates, each
  workflow step to the function that does it and the tests that cover it, and
  the paths no task may change.

## Checks

- `crates/implementation/tests/dogfood.rs`: the model is valid under
  Agentique's own language core, its crates follow its dependencies, and
  every workflow scenario runs and passes in model execution.
- `tools/check_architecture.py` compares the model with `cargo metadata` and
  prints one line per problem when a workspace crate is not mapped to a part
  (or a mapped crate is missing), when a crate's normal or build dependency on
  another workspace crate is not allowed, when LanguageCore depends on
  another part, when a crate of the locked core, the Library, Simulation,
  Implementation or Execution uses a UI or network library, or when a crate
  outside Providers uses rig, tokio, reqwest or the credential store (R-41,
  ROADMAP §8.7; temporary exceptions are listed in the script). It reads only
  part defs, their `Crate` parts and `dependency` statements, and reads past
  everything else; dev-dependencies are ignored. CI runs it on every pull
  request.

```sh
python tools/check_architecture.py
python -m unittest discover -s tools -p test_check_architecture.py
cargo test -p agq-implementation --test dogfood
```
