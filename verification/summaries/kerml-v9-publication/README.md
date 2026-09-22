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

The first A preflight completed producer closure and all 1,173 mandatory
references, including `Transfers::Transfer::instant[instantNum]`. It exposed
two separate integration defects: the capability audit applied the strict
binding-endpoint contract to abstract `Transfers::transfers`, and its scoped
producer population omitted dependencies discovered by the final graph and
queries. The published Connector related-feature projection now filters absent
references; this adds no operational correction. Slices expand discovered
providers and restart from the declared graph, with a recorded development cap.
Acceptance requires rerunning the preflight after both fixes.

The third A preflight passed under query rule set `/26`: 11,227 declared scope
subjects, 21,656 audited subjects, 1,629 Complete mandatory references, zero
capability/reference findings and zero missing providers. Its three scope
attempts took 817.27 seconds after shared preparation (203.31 seconds). This is
larger work than the previous controlled gate: scope expansion and complete
capability/reference audits are included. Peak private memory through A was
2,212,048,896 bytes, below twice the earlier controlled 1.75 GiB observation.
The unchanged synthetic gate below separately checks producer performance.

[connector-related-checks.json](connector-related-checks.json) records the exact
published rule and focused checks. [scale-checks.json](scale-checks.json) records
the retained 60,000-subject workload at 5.83 seconds and 933.3 MiB peak private
memory, effectively unchanged from 5.82 seconds and 932.3 MiB. Scope-expansion
work is reported separately from producer performance.

[commands.json](commands.json) records actual commands, source identities, exit
codes, and output hashes. Raw logs and monitor samples remain ignored under
`verification/generated/`. The current reference source capture is a maintenance
action, never a build input or an artifact acquisition step during a build.
