# Operational v9 and canonical KerML publication

Base: fetched `origin/main` at `150d97879b84ea3ebc9978256a5c93e391602244`.
The initial worktree was clean. Implementation branch:
`foundation/kerml-v9-to-sysml-semantics`.

This milestone adds exactly the two project-authorized multiplicity context
interpretations in [ADR 0023](../../../docs/adr/0023-operational-multiplicity-context-v9.md).
Published through v8 remain historical profiles. The default operational alias
remains frozen until canonical publication acceptance.

[authority-decision.json](authority-decision.json) records exact pinned formal
rules, current issue and immutable reference implementation identities, corpus
witnesses, and retained historical findings. Its zero authority blocker count
does not assert producer closure or publication acceptance.

Implementation and acceptance are in progress. A–E slices must pass in order
before one monitored whole-corpus attempt (90 minutes, 6 GiB private memory).
Conformance coverage is incomplete and independent of publication acceptance.

[commands.json](commands.json) records actual commands, source identities, exit
codes, and output hashes. Raw logs and monitor samples remain ignored under
`verification/generated/`. The current reference source capture is a maintenance
action, never a build input or an artifact acquisition step during a build.
