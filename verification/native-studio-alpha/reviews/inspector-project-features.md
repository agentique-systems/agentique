# Inspector: project features before standard semantics

The real `real-run02/gallery/02-focused-subsystem.png` and
`02a-platform-port.png` show inherited standard features filling the primary
Ports & Interfaces section. On `clientQueries`, this pushes the actual
`queryConnection` relationship below repeated Connector and standard port rows.
The default list therefore fails the engineering-first Inspector review.

This correction adds a revision-bound `ElementInspector.feature_provenance`
map for owned and effective canonical features. Exact accepted-library ID
membership takes precedence over the canonical record's origin. Declared-name
presence uses the same resolved canonical property lookup as the projection;
it does not mistake a metaclass label fallback for an engineering name. Source
availability comes from the inspected revision's fact-to-source mapping.

Primary sections show named authored features. Effective ports and interfaces
retain their canonical IDs and explicit Inherited marker. No scene membership
or current projection depth participates, so an authored inherited feature such
as `repositoryRevisions` remains listed before it enters the visible scene.
Selection behavior is unchanged: the current journey enters ModelRepository
before selecting that port. Owned features with unavailable provenance remain
visible and are labelled Origin unknown. A source location is optional for an
otherwise known authored record.

Effective semantics retains both complete owned and effective lists, with
known origins displayed; query summaries and exact identities remain available.
No canonical records, inherited ownership, query results, completeness or
accepted runtime identities are changed.

The native classification regression includes authored features named Connector,
a standard feature named repositoryRevisions, missing owned/inherited provenance,
unnamed features, derived/generated features, duplicate IDs and equal names.
It checks borrowed identity and that both unfiltered input lists remain intact.
The modeling-view regression reads real canonical fixture slots and checks exact
standard-ID classification, undeclared-name fallback, Unicode short names and
missing IDs. These are bounded classification tests, not accepted-runtime tests.

Focused rustfmt and diff checks are recorded in
`checks/inspector-project-features-source.json`. Compilation, test execution and
the new real-model screenshots are deferred to the integration lead to avoid
concurrent builds and runtime consumers. No visual acceptance is claimed here.

Integration checks to run:

```powershell
cargo test -p agq-modeling-view --lib feature_metadata_uses_exact_standard_identity_and_canonical_declared_names
cargo test --manifest-path crates/studio-native/Cargo.toml --target-dir target/native-alpha --bin agq-studio-native inspector::tests
```

The existing `ElementInspector` test literal in `real_automation.rs` is updated.
The integration branch's newer `revision_reads.rs` test literal and the editor's
new concurrent-read driver test literal also require
`feature_provenance: Default::default()` when this commit is integrated.
