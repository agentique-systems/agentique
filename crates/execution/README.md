# agq-execution

Execution (ROADMAP §4.15; part `Execution` in
`models/agentique/Agentique.sysml`): every side effect outside the System
State, as typed operations with a scope. It depends on no other part of
Agentique.

```rust
let scope = Scope::writable(&worktree, &["src", "tests", "Cargo.toml"], &["tests/contract.rs"])?;
let executor = Executor::new(scope).trusted(project_allows_it).target_dir(per_copy_target);
executor.write("src/api.rs", text)?;                          // refused outside the scope
let finished = executor.run(&Program::cargo(&["test", "--offline"]), "", timeout)?;
```

- **Scope.** Paths are relative to the root and canonicalised. Absolute
  paths, drive and UNC prefixes, `..`, alternate data streams, reserved
  device names and anything a link leads out of are refused. Writes go only
  under the allowed paths (an empty entry: anywhere in the root, as for a
  task's worktree) and never under protected ones (contract tests,
  scenarios, evaluation cases, the model folder), so a task cannot make a
  failing check pass by weakening it.
- **Commands.** Cargo only, and only `build`, `check`, `test`, `run`,
  `metadata`; no `--config`, toolchain overrides or manifest redirection.
  The environment is rebuilt from a short list (PATH, temporary folders,
  Cargo and rustup homes, ...) and anything that looks like a key, token,
  secret, password or credential is dropped. Cargo runs offline unless the
  network is allowed. A timeout or cancellation ends the whole process tree
  (`taskkill /T` on Windows). Output is captured and capped.
- **Isolation.** `Isolation::describe` says it plainly: trusted-local, no
  sandbox. Nothing runs unless the Operator turned trusted-local execution on
  for the project. A worktree isolates edits, not processes.
- **Git** (`git`): a task's worktree on its own branch from a base commit;
  the patch it made; integration that refuses when the head moved or when a
  patched file has uncommitted changes, and never touches other uncommitted
  work. Uses the same embedded git as History, without network features.
- **Jobs** (`jobs`): stable ids, states (pending, running, waiting for you,
  done, failed, refused, cancelled, interrupted) and a journal written before
  and after each side effect. Opening a project marks jobs left running as
  interrupted, with which steps started and never recorded their end.

Each working copy (the main tree and every worktree) builds in its own
target folder: two copies of one crate sharing a target folder can reuse
each other's stale builds.
