# Independent audit-status and Graph-label source review

Reviewer: editor interaction / semantic-boundary reviewer, 2026-09-26.
This is a source review. The reviewer ran no build, test, native process or
accepted-runtime consumer.

## Audit probe expectation

Reviewed correction `0bd685cebfaf5d47730f6a87136727679d7fec34` after the
actual parity test failed at `publication_audit_tests.rs:147`. The retained
first-run receipt records exit 101, 127.773 seconds measured wall time and
5,831,131,136 bytes peak memory. The test itself reported 126.96 seconds.
Executable SHA-256:
`4ca436f8278f429b48be0026cf7f1433589187a7da38dd336b8c3301a781c130`.
The failure occurred in the ordinary-source missing-subject probe before the
variation-source iteration. It does not qualify the complete oracle.

The original expected `Incomplete` was incorrect for the composed public
query. The unchanged call chain is:

1. `SysmlQueries::effective_usages` calls `current_effective_usages`
   (`crates/sysml-semantics/src/queries.rs:479`, `:502`).
2. That wraps `KerMlQueries::effective_features`, whose first operation is a
   checked typed `Type` read (`crates/kerml-semantics/src/inheritance.rs:10`).
3. `checked` passes the typed-view error into `accept`. Missing element is
   outside the enumerated pending-computation/construction cases and therefore
   becomes `Invalid` (`crates/kerml-semantics/src/queries.rs:201`–`:248`).
4. The SysML domain check additionally retains `SQ_MISSING_ELEMENT` with
   `Incomplete` (`crates/sysml-semantics/src/queries.rs:208`–`:217`).
5. Public `SysmlQueryResult::completeness()` takes the maximum of the composed
   KerML answer, additional status, supporting queries, names, observations and
   pending implications (`crates/sysml-semantics/src/queries.rs:78`–`:95`).
   The public answer therefore remains `Invalid`.

The correction changes only the probe expectation and explanatory comment,
adds an explicit assertion of the composed KerML `Invalid` status and the
exact missing-subject diagnostic, and improves failure output. It retains
whole-answer Debug equality, exact capability-diagnostic equality, exact audit
report equality, and one observer delivery. Ordinary `Complete`, wrong-kind
`Invalid`, and genuine variation `Incomplete` remain separately asserted.
The commit changes no production query or audit semantics. Source judgment:
correct narrow test correction; the complete actual-runtime oracle still
requires a successful rerun.

## Graph relationship annotation

Reviewed `217d99274d67a325d9bcb051340ecce21973b937` and header followup
`cca9eab672bff739faa4c310188a423960f785b8` against the actual run05 Graph
criticism: the six-node scene at summary zoom hid the selected Repository's
relationship labels despite its small neighborhood.

The implementation permits selected-incident labels at Summary only in a
bounded visible graph (at most 24 edges), with eight automatic labels. Explicit
selection and hover retain priority. Text and derived markers come from the
actual edge DTO. The placement helper clips route segments to the visible
viewport, tries a bounded set of anchors, and avoids card/header rectangles and
previous higher-priority labels. A leader ties displaced text to its real route.
Off-screen routes produce no invented in-view anchor. Dense automatic placement
may omit a label; explicit inspection retains a visible fallback and the
existing Inspector.

Initial review identified a concrete System-view regression risk: the old
58-world-unit title obstacle did not cover the new port header rows. Followup
`cca9eab` derives each owner's obstacle bottom from its actual visible
`label_in_header` ports plus 12 screen pixels, bounded by the container. The
ordinary title bottom remains the minimum. This resolves the identified
header-row overlap in source without fabricating layout semantics.

Remaining qualification: run the prepared geometry tests and capture the actual
Graph and selected-port scenes. Source inspection does not establish final text
density or eliminate general dense-graph label collisions. No GPU, frame-rate,
input-latency or new visual-acceptance claim is made here.
