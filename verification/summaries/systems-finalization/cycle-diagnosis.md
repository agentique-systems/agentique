# Final effective-query cycle diagnosis

The read-only Systems finalization failed its effective SysML population audit.
The retained diagnostic set contains 21 distinct `KQ_RESULT_INHERITANCE_CYCLE`
subjects and three distinct `KQ_END_CYCLE` subjects. This investigation did not
run producers, edit the retained frontier, change the registry, or confer
acceptance. No complete cycle fix was implemented or tested.

## Examined evidence

The investigation streamed `kernel.jsonl` from the accepted KerML v9 cache and
`graph.jsonl` from retained frontier archive
`c42d679d2908df7ba0a864710eda0e4e8c8cb97e70ead2ef5e9c604888bd447d.zip`.
It projected 75,342 final canonical records: 61,718 accepted KerML records and
13,624 local records. Archive origins and proof payloads were not copied into
the diagnostic projection. The source remained unchanged.

Generated inputs and projections are ignored storage:

- Lead checkout: `verification/generated/systems-finalization/findings.json`
  and `finding-subjects.json`.
- Isolated diagnosis checkout: `verification/generated/cycle-audit/inspect.py`,
  `records.json`, `cycles.json`, and `simulate.py`.

`python verification/generated/cycle-audit/inspect.py` and
`python verification/generated/cycle-audit/simulate.py` both exited 0.
These are diagnostic scripts, not semantic acceptance tests. No Rust build or
test ran in this workstream while the lead owned the compiler resource window.

## Return populations

`SysmlQueries::effective_return_parameters` in
`crates/sysml-semantics/src/structural_queries.rs` explicitly accepts all
Definitions and Usages. Limiting the publication audit to Functions or
Expressions would hide failures within that public API's current domain.

`KerMlQueries::compute_result_parameters` in
`crates/kerml-semantics/src/ordering.rs` first discovers reachable owned return
memberships, then requires a topological evaluation order. A parent cycle
currently produces `KQ_RESULT_INHERITANCE_CYCLE`, including when every observed
owned return population is empty.

All 21 failing subjects have zero owned `ReturnParameterMembership` records
throughout the reachable stored specialization/conjugation/chaining projection;
their projected reachable populations range from 10 to 31 records. Examples:

| Subject | Metaclass / declared name | Reachable records | Return memberships |
| --- | --- | ---: | ---: |
| `515638fd-0ff9-53fc-a6ba-05e1d820e62c` | ReferenceUsage / unnamed | 12 | 0 |
| `888cda79-aaa5-5d72-a654-b35603802e37` | OccurrenceUsage / source | 14 | 0 |
| `5e756f07-e2c7-56e7-8a4c-7b5283d66543` | FlowUsage / successionFlows | 31 | 0 |

An empty result population can in principle be proved from exhaustive negative
membership evidence without inventing canonical facts. Any implementation must
retain all prerequisite evidence and incomplete or invalid status, preserve the
accepted KerML interpretation, and test producer interactions because internal
producer queries also call `result_parameters`. This static investigation does
not establish such an implementation's compatibility with the retained
certificate.

## End populations

The three end-cycle subjects are:

- `18da1d9a-37e3-547a-ac0e-bf9793f724b4`: FlowUsage `messages`.
- `3b9fab91-096f-5012-841c-ad946c367fdf`: FlowUsage `flows`.
- `5e756f07-e2c7-56e7-8a4c-7b5283d66543`: FlowUsage `successionFlows`.

The stored projection contains the cycle `messages -> flows -> messages`.
Both members own two ends. External general end vectors in the simulation
contain at most two ends; `successionFlows` owns two ends and depends on `flows`.

`structural_positioned_features` and `complete_owned_position_cycles` in
`crates/kerml-semantics/src/implicit.rs` already contain a conservative
composed-language cycle proof: equal owned position counts must cover all
resolved external general populations. The static projection satisfies this
condition, and the diagnostic simulation resolves all three subjects. This
does **not** explain the runtime findings. Further exact query-context and
property-read tracing would be required; relaxing the existing cycle guard is
not justified. Existing `owned_position_cycles.rs` regressions deliberately
retain incomplete answers for uncovered cycles and sealed KerML-only contexts.

## Limits and disposition

The Python projection reads stored records and descriptor inheritance/redefines
metadata. It is not the real evaluator: it does not restore the semantic context,
execute query-time library implications, reconstruct all association-occurrence
reads, evaluate visibility or explicit redefinition evidence, or authenticate
query completeness. The observations narrow the investigation but do not
replace the failing full audit.

The separate final audit also found enumeration typing facts absent from the
retained graph. Resolving a read-only cycle query would not supply those facts
or authorize publication. Systems acceptance remains incomplete.

The prepared ProjectWorkspace commits
`427d29b2633d5ac1183e0338df96d4ef4300552e` and
`1ee7be295f43266c7f480b78865f3fd60a0874e9` remain held on
`platform/workspace-integration`. They must not be integrated or represented as
accepted workspace delivery before the Systems and language readiness gates
pass.
