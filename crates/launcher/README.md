# agq-launcher

The launcher (ROADMAP §4.15, C-51; part `Launcher` in
`model/Agentique.sysml`): Agentique's installed builds, which one starts,
and the last known good one. It depends on no other part of Agentique, so
it works when a new build does not, and it never builds, downloads or
deletes anything.

- **Registry** (`builds.json` in `%LOCALAPPDATA%\Agentique\builds\`, or the
  folder `AGENTIQUE_BUILDS` names): the builds, the current one and the last
  known good one, saved atomically; a later format is refused.
- **Manifest** (`build.json` in each build's folder): the repository,
  commit and tree it was built from, the task, the toolchain, the Claude
  Agent companion and its packages, the required checks it was built after,
  the data formats it reads and writes, and the SHA-256 of each executable.
  A build whose executables no longer match is not started.
- **Starting** (`start`): the build gets `--ready-file`; it writes the file
  once its window is up. A build that exits first, or is not ready within two
  minutes, is marked failed with the reason (its error output in
  `start-failure.log`), and the last known good build starts instead with
  `--recovered-from`, so it can say what happened. If that fails too, the
  builds folder opens with `launcher.log`.

```text
agentique-launcher [args…]                       the current build
agentique-launcher --recover [args…]             the last known good build, in safe mode (no Claude Agent runtime)
agentique-launcher --adopt <id> --wait <lock>    after the running Agentique hands over (Use this build)
```
