# Operational v9 publication tooling

The release runner requires all five v9 slices before starting whole-corpus closure.
Slice evidence pins the profile, semantic rule set, descriptor digest, exact source
content set and declared graph digest. A report also requires Complete producer
closure, a complete scope boundary, zero capability/reference findings, a complete
symbolic-bound audit and no resource stop. Slices always stop at the first failure.
Shared source preparation and each slice have separately reported elapsed times.

Build once with debug information and incremental output disabled, using the shared
workspace target directory. The lead runs the integrated branch, so `CARGO_MANIFEST_DIR`
identifies the integrated corpus/evidence root:

```powershell
$env:CARGO_PROFILE_RELEASE_DEBUG = '0'
$env:CARGO_PROFILE_RELEASE_INCREMENTAL = 'false'
$env:CARGO_PROFILE_RELEASE_LTO = 'false'
cargo build --locked --offline --release -j 2 -p agq-kerml-text --example publication_slices --example canonical_publication --example multiplicity_inventory
```

Inventory exact source without invoking publication closure:

```powershell
target/release/examples/multiplicity_inventory.exe --output=verification/generated/kerml-v9-publication/multiplicity-inventory.json
```

The inventory classifies ordinary/cross-feature symbolic bounds, literal bounds,
unbounded `*`, and other expressions. It traverses canonical ownership to include
nested FeatureReferenceExpressions, and cross-checks all 15 retained reference
identities (9 ordinary, 6 cross) from `remaining-structural-investigations.json`.
Slice and accepted-publication reports additionally audit symbolic featuring and
reference-result binding contexts. No bounds are numerically evaluated.

```powershell
python verification/scripts/watchdog.py --name kerml-v9-slices --wall-seconds 1800 --private-mib 3500 --summary verification/summaries/kerml-v9-publication/commands.json -- target/release/examples/publication_slices.exe --slice=all --output=verification/generated/kerml-v9-publication/slices/slice-{slice}.json
```

If any slice fails, fix that failure before proceeding. Review changes against the
controlled v8 slice (~269 seconds including preparation; 1.75 GiB peak private).
The memory cap is twice that observed private-memory baseline. The combined wall
cap allows the shared preparation plus five sequential slices; it does not replace
reviewing the individual elapsed times against the historical measurement.

Only after A–E pass, run the one monitored whole-corpus attempt:

```powershell
python verification/scripts/watchdog.py --name kerml-v9-canonical --wall-seconds 5400 --private-mib 6144 --summary verification/summaries/kerml-v9-publication/commands.json -- target/release/examples/canonical_publication.exe --slice-evidence=verification/generated/kerml-v9-publication/slices --output=verification/generated/kerml-v9-publication/canonical.json --write-bindings --conformance-output=verification/generated/kerml-v9-publication/conformance.json
```

The accepted facade is reused for binding format `/2`, its structural stale check,
all retained multiplicity witnesses, and two authored SourceProjects. The latter
checks imports, aliases, specialization, typing, subsetting, redefinition, visibility,
shadowing, accepted library feature chains, stable IDs, immutable parallel readers,
edit isolation and an explicitly wrong profile. Authored chain syntax is a separate
frontend capability; the chain check here proves that authored contexts reuse the
accepted canonical chain records and semantic ordering.

Focused verification passed seven library tests and the stale/incomplete preflight
regression. The source inventory passed with 1,498 MultiplicityRanges and 2,096
bound Expressions: 9 ordinary symbolic, 6 cross symbolic, 1,716 literal and 365
unbounded. All 15 retained witness identities remain present. These source counts
do not claim semantic publication acceptance.

The sibling-worktree release build was deliberately stopped (exit 124,
`stop=requested`) so the lead could build in the integrated main worktree; example
binaries embed `CARGO_MANIFEST_DIR`. This was a build cancellation, not a corpus
attempt or semantic failure.

Raw outputs remain ignored. `tooling-commands.json` contains actual focused-check
commands, exit codes and output hashes; publication acceptance is reported by the
lead only after the monitored gates complete.
