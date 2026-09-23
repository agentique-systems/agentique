# Anonymous Flows end closure regression

Base `2ed6566`; no accepted cache or corpus run. The bare source preflight loads
the pinned source bytes, parses and constructs Flows without producer evaluation.
It passes for both remaining anonymous ends (`63d0949c…`, `86ddb7cd…`, 0.33 s).
Each is a noncomposite ReferenceUsage under EndFeatureMembership of ConnectionUsage.
Its ordered children are OwningMembership containing a plain Feature for `[1]`,
then ReferenceSubsetting to source/target. The plain Feature owns MultiplicityRange,
which owns LiteralInteger(1) and its implied plain Feature result. The multiplicity
is not a direct child of the end.

The genuine closed-dependency end fixture now adds that exact ownership/class
pattern, reference endpoints, a typed anonymous connection and positional ends.
The second end (`76012`) reproduces one incomplete producer pair and an open
EffectiveTyping requirement; the first end without the owned multiplicity closes.
Actual red: `anonymous_connection_end_with_owned_cross_multiplicity_closes`,
exit 101, 17.29 s. Expected Complete/full-certificate assertions remain unchanged.
The existing named/redefined end fixture is retained through a shared test runner.

The independent architecture workstream owns the generic
`owned_cross_subsetting` population correction. This commit contains no semantic
production change. Initial diagnostic compilation used an incorrect registry
method and nonexistent generic Usage portion property; corrected. The first
synthetic attempt wrote the derived referencingFeature endpoint; corrected to
ordinary owned ReferenceSubsetting with its stored target before the semantic red.
All commands and raw hashes are in `flows-end-commands.json`.

## Corrected gate

Independent source `792a5ca` (local `9a05c1e`) changes only the owned
cross-subsetting lookup from all owned relationships plus an in-memory class
filter to the existing typed CrossSubsetting population query. First canonical
order, subtype eligibility, completeness and the explicit pending-specialization
guard remain intact. The architecture workstream owns the planner causal red and
the direct/future eligible-writer and provider controls. Read-only review found
no additional semantic change or loss of an eligible carrier.

| Check | Actual result |
| --- | --- |
| `cargo test --locked --offline -p agq-sysml-semantics end_usage_tests -- --nocapture --test-threads=1` at `9a05c1e` | Exit 0; both old named/redefined and new anonymous multiplicity fixtures pass, 33.54 s test time. Full graph certificate closure, expected owned-cross selection and true mayTimeVary/isVariable assertions pass. |
| `cargo test --locked --offline -p agq-kerml-text --lib flows_anonymous_connection_end -- --nocapture --test-threads=1` at `9a05c1e` | Exit 0; 1 passed, 0.35 s. Exact pinned frontend identities, classes, order and multiplicity shape match the fixture. |
| `cargo fmt --all -- --check` at `6100dd0` | Exit 0 after sorting the two test module declarations. The initial format-only failure is retained. |
| `cargo clippy --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` at `6100dd0` | Exit 0, 20.42 s build time. |

`6100dd0` is source-only formatting; this final update is evidence only. All
compilation used one job, no incremental/debug artifacts and the isolated
`target/bridge-closure` directory. No accepted cache, corpus or publication was
loaded or run. This establishes the bounded regression gate, not acceptance of
the still independently gated standards publication.
