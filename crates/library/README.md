# agq-library

The Library of building blocks (ROADMAP C-49, §4.13; part `Library` in
`model/Agentique.sysml`). A building block is any reusable
definition, from three scopes: the built-in blocks in
[`blocks/Library.sysml`](blocks/Library.sysml) (plus the standard
`ScalarValues`), the open project's own definitions, and My Library, the
Operator's blocks kept in one SysML file in the app's local data.

```rust
let library = Library::with_mine(path);                 // or Library::built_in_only()
let index = Index::build(&library, Some((state.tree(), state.revision())));
let hits = index.search(&Query { text: "cache", ..Default::default() });
let plan = library.plan_use(&state, &Use::new(block, Parent::Element(system)), Actor::Operator)?;
state.apply(plan.change)?;                              // the Studio applies it like any change
```

- **Index** (`index.rs`): one summary per definition, read from the model:
  name, category (package), purpose (`doc`), ports (owned and inherited, with
  the side they are drawn on), parts, attributes, requirements; for the
  project, usages and where a copy came from. Refreshed per project revision
  and My Library version, so drawing never walks the model.
- **Search** (`search.rs`): a fuzzy matcher that prefers word starts and runs
  and returns the matched characters; filters by scope, kind and a set of
  blocks (such as those that fit a port).
- **Copy** (`copy.rs`): a block's dependency closure, with the scenarios
  whose subject a copied definition types (a block carries its behaviour and
  the scenarios that show it, C-50); the comparison that finds identical
  copies; the plan of new elements with every reference, those in
  expressions and `via` included, pointing at its target by identity.
  Conflicts are reported, never overwritten.
- **Plans** (`plan.rs`): using a block (copy, usage, values, connection),
  specialising, overriding by redefinition, extracting a block from a
  selection, saving to and removing from My Library. Each project change is
  one System State `Change`, tried on a copy before it is returned.
- **Compatibility** (`compatible.rs`): which blocks fit a port, decided by
  `agq_language::Semantics::ports_fit`, the model's own rule.
- **Descriptions** (`describe.rs`): a block in words for the Assistant, and
  the structural preview the Studio draws.

The built-in blocks are validated by the tests; keep them few, neutral and
documented (§1.3). `Resilience::RetryingWorker` and `Moderation` (an agent
with a deterministic fallback) carry state machines and scenarios that run.
