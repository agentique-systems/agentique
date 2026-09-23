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

After exact append support, an initial derived ordered-slot case remains:
`cargo test -p agq-kerml-semantics --lib initial_derived_ordered_entries -- --nocapture`
exited 101 (one failing test, 0.16 s). A newly created chain Feature initially
owns both chaining records. The query returns the correct ordered targets and
typed population search, but additionally retains the whole ownership Property
read because those initial entries did not pass through append bookkeeping.
This reproduces the remaining receiver cycle without any SysML fixture.

Corrected production (`fa3bbdd` + kernel `4fa87cd`) and fixture assertions
(`dc888fd`) pass:

- `cargo test -p agq-kerml-semantics --lib selected_contribution_merge -- --nocapture`:
  exit 0; 2 passed, 0.17 s. Both evidence modes and merge orders retain explicit
  broad reads; initial created entries retain their typed population search.
- `cargo test -p agq-kerml-semantics --lib selected_contribution_tests -- --nocapture`:
  exit 0; 3 passed, 0.28 s. Selected guard changes reopen retained evaluations;
  archives without optional precision conservatively use aggregate proof.
- `cargo test -p agq-sysml-semantics`: exit 0; 73 passed, 71.38 s, docs passed.
  Includes the receiver, assertion body, while-loop body, nested transition,
  owner/type negative cases, and unsupported variation remaining Incomplete.

The optional contribution map affects checkpoint revalidation precision, not
canonical graph identity or accepted complete-publication truth. No cache load
or corpus publication was performed by these commands.
