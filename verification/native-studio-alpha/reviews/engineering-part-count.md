# Engineering part counts

The actual `real-run03/gallery/02-focused-subsystem.png` shows ModelingPlatform's
header reporting 9 parts while its Inspector correctly lists eight parts and
the queryConnection interface. Canonically, ConnectionUsage and InterfaceUsage
are PartUsage subtypes; a raw PartUsage count therefore mixes engineering roles.

`FeatureCounts.parts` now excludes canonical Connector subtypes. The complete
feature list, canonical IDs, original order, ownership and interface display
are retained. Port/requirement counts and language semantics are unchanged.

The regression `engineering_part_count_excludes_connectors_without_losing_owned_features`
uses the canonical SysML registry and original ownership memberships. It checks
a true PartUsage named Connector plus InterfaceUsage and ConnectionUsage, verifies
their real subtype relationships, counts one engineering part and retains all
three original features and owners. No name matching determines classification.

Focused source-format and diff checks are recorded in
`checks/engineering-part-count-source.json`. Tests/builds/runtime were not run
in the isolated worktree. Integration can run
`cargo test -p agq-modeling-view --lib engineering_part_count_excludes_connectors_without_losing_owned_features`
and capture the corrected real header; this source-only record does not claim
that new screenshot or test result.
