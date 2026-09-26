# Portable Windows helper build and real project creation gate

GitHub Actions [run 36234518201](https://github.com/agentique-systems/agentique/actions/runs/36234518201)
built the default thin-LTO release helpers from exact source
`fea3339dbf0e84d408cb7d47c802a7b047826f1f`. The retained build manifest, command
logs and source manifests bind all three executables. Downloaded binary hashes
were checked against that manifest before use. Executables are excluded from
this directory and remain in the original CI artifact.

`project_creation.exe` SHA-256:
`6b5b4296e6e6e3a6f33e99b732c58d83f10a4ae6e4c498b4f5b8a3ebc2fb9be9`.

The operator machine ran its explicit ignored integration test against the
installed real accepted runtime, with `AGENTIQUE_SOURCE_ROOT` set to this
checkout and `AGENTIQUE_RUNTIME_DIR=C:/Users/phili/.agentique`. The actual command,
stdout/stderr, exit code and before/after executable hash are retained in
`../../native-studio-alpha/checks/acceptance-project-creation-01.{json,txt}`.
It passed in 324.297 s including runtime load and multiple source constructions.
This is a functional gate, not a project-creation latency measurement; a semantic
unit build/test ran concurrently during part of this invocation.

The test creates a new repository and empty Working project; verifies that invalid
names, relative paths and unauthorized creation leave no repository; imports exact
source bytes; rejects unvalidated commit; cancels without changing source, head or
revision count; validates and commits a subsequent import; rejects stale and
duplicate source proposals; and confirms a failed repository switch preserves the
current host. It authenticates the actual publication facade rather than treating
fixture data as runtime authority. This backend lifecycle gate does not by itself
qualify the interactive New Project dialog.
