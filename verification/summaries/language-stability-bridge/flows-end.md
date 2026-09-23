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
