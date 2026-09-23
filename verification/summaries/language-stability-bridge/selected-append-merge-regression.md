# Selected append evidence and broad read composition

The independent regression appends a Subsetting and an unrelated FeatureChaining
to the same owner, with distinct declared support facts. The selected Subsetting
query must exclude the sibling contribution. A caller that also requests the
whole current ownership fact must retain that sibling support and the broad
Property read in either merge order, in ordinary and producer evidence modes.

Before the exact contribution correction:
`cargo test -p agq-kerml-semantics --lib selected_append_and_whole_slot -- --nocapture`
exited 101 (one failing test, 0.16 s). The selected-only answer incorrectly
contains the sibling support fact at the first exclusion assertion. This is
a reproducer, not an acceptance result; integrate with its production fix.
