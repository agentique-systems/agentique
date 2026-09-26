# Isolated retained derived frontier experiment

This branch experiments with retaining exact local derived facts across the
golden CreatePartUsage edit. It is based on main `42e7e187`, whose measured native
candidate still took 129.738 seconds. No faster candidate or exact real-model
equivalence is claimed for this draft.

The parent is immutable. `AGENTIQUE_RETAIN_DERIVED_FRONTIER=1` is consulted only
by a verification build, only without a semantic-cache restoration and without
an intermediate construction certificate. Production builds cannot activate it.
The ordinary final producer scheduler, effective audit, candidate validation,
durable commit and independent cold oracle remain required.

The kernel reconstructs a requested subset of the old local derived overlay on
the new strict declarations. Both the descriptor registry and immutable
dependency must have the exact same allocation identities. New declarations
are authoritative: no old declared record is copied over them. Changed positive
declared support, missing derived support, failed computations and incomplete
created records are retracted to a fixed point. Created records remain atomic.
Remaining records, proof edges, negative searches and ordered contribution
evidence keep their original bytes. Strict model construction rechecks required
bounds, endpoints and dependency ownership. A subset of the checked old proof
DAG cannot introduce a cycle. This is structural validity, not language truth.

The language pass computes exact old/new fingerprints itself; callers cannot
submit a claimed affected set. It checks actual immediate positive facts, with
the kernel handling transitive support; compares negative search populations;
and recomputes producer closure using the existing checked checkpoint/rebind
contract. All six current causal closure requirements must hold on each positive
read subject and on general bounded search subjects. Unknown search contracts
and genuinely global changes fail closed. Fixed identity-only and original
declared-property searches retain their documented narrower meaning. Removing
any output triggers another kernel and causal pass until no further output is
removed. A merely provisional overlay is never used to execute a producer or
answer an ordinary UI query. Ordinary scheduling starts only from this checked
fixed point and still evaluates every reopened family.

This deliberately conservative algorithm may retract most outputs. It may also
cost too much through repeated certificate/context construction. Both outcomes
must be measured, not inferred from retained counts. The experiment emits
`SOURCE_RETAINED_FRONTIER_EXPERIMENT` with elapsed time, original/retained fact
counts, retraction rounds and retained/reopened evaluation counts. Existing
source phase timings and audit rejection diagnostics remain enabled.

Nine initial regressions cover exact unchanged records/proofs/searches, changed
positive and transitive support, partial created records, ordered contribution
preservation and changed prefixes, failed computations, separate dependency or
registry mounts, new negative incoming matches, rename, newly applicable
potential writers, and unknown/global searches. They have not yet been compiled
or executed locally because the integration compiler slot is occupied. Formatting
and whitespace checks are recorded separately. A read-only independent review
has been requested before any production promotion.

The isolated CI router input `experiment_retained_frontier=true` skips native,
helper and prior-baseline builds. It builds the release oracle, executes the
focused kernel and semantic regressions, and only if those pass runs the current
command/cold independent process oracle with the experiment enabled. The cold
child has no parent revision and reconstructs from source. Every semantic
observation still compares exactly; no graph-count shortcut is introduced.
The authenticated runtime transport, build/executable/source receipts, process
memory and logs are retained on success or failure. No ordinary release is
published by this experiment.
