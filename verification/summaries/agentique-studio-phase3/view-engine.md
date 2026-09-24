# View engine verification

The exact focused commands, outputs, elapsed times and exit codes are recorded in
[view-engine-commands.json](view-engine-commands.json).

All four checks passed: package formatting, warning-free Clippy across all
targets, five focused tests, and Rustdoc. The accepted-cache integration binary
compiled and its one explicit cache-dependent test was ignored. No accepted-cache
semantic integration success is claimed by those five unit tests.

Review added a sixth test over a strict canonical SysML Snapshot containing real
OwningMembership, FeatureMembership and FeatureTyping records, PartDefinition,
PartUsage, PortUsage and RequirementUsage. The new test first reproduced a missing
FeatureTyping endpoint when its inherited property alias was read directly; see
[the failed regression](view-engine-property-resolution-regression.json). All
projection property reads now resolve effective metamodel replacements before
kernel navigation. The corrected test checks canonical edge identities, owners,
qualified names and direct feature counts. The final package checks and six-test
result are in
[view-engine-property-resolution-commands.json](view-engine-property-resolution-commands.json).

The opt-in gate is:

```text
cargo test -p agq-modeling-view --test accepted_views -- --ignored --nocapture
```

Its scope is one authored system fixture plus a second tiny document/revision. It
restores the exact accepted publication files selected by `AGENTIQUE_KERML_CACHE`
and `AGENTIQUE_SYSTEMS_CACHE`; missing files fail immediately. It checks real
projection, selected-element effective queries, exact owner source ranges,
canonical derived evidence, endpoint identity, requirement absence, retained old
revision answers and view-only hiding. It never initiates standard publication.
