# SysML semantic foundation

`agq-sysml-semantics` composes `agq-kerml-semantics` over the same immutable kernel
graph. There is no parser dependency, second model store, copied inherited Usage,
or SysML-specific element identity.

Production contexts require the actual accepted KerML `CompletePublicationOverlay`,
the exact protected snapshot/construction dependency, and an expected
`SysmlDependencyContract`. The compiled acceptance receipt independently establishes
the KerML publication digest, v9 profile, rules, library set, source identity and
descriptor graph. The combined SysML descriptors, SysML rule set, Systems Library
identity and requested standard bindings are also checked. A mismatch is an error.
An unpublished Systems candidate never becomes an accepted dependency through this
API.

`SysmlQueries` supplies direct typing, current derived typing, specialization,
reflexive supertypes, owned and inherited usage sets, subsetting, redefinition and
ordinary naming. Attribute/Item/Part projections reuse KerML's algorithms. Usage
typing retains valid KerML Classifiers; Attribute typing accepts DataType and Item
typing accepts Structure. Invalid typed projection endpoints remain in
`rejected_targets` with diagnostics. Port typing and ConnectorAsUsage related
features are structural projections only.

Query results retain the composed KerML evidence. Additional observations retain
canonical origins and proof/search dependencies. The effective usage collection
is an identity set, not normative feature order. Qualified names retain ordered
segments with factored long/short-name choices; they do not expand an exponential
number of paths.

Current-graph queries can be complete without claiming SysML producer closure.
Effective queries expose missing Item/Part base relationships, uncomputed derived
`mayTimeVary`, variation/individual implications and the remaining producer-closure
obligation. Composite Item subsetting remains blocked by the unapproved final-rule
`subitem` versus pinned-library `subitems` discrepancy. Nothing here adopts that
correction, defaults `mayTimeVary` to false, or certifies Systems Library publication.

`StandardSysmlBindings` validates only requested Item/Part anchors, requiring exact
owned paths, unique public declarations, exact metaclasses and source library
provenance. Attribute anchors reuse the accepted KerML bindings. Unbound roles
remain explicit; the bindings API does not bind or validate the whole library.

Unit tests use an explicitly private synthetic context constructor. Production
textual and programmatic integration must use the actual accepted publication.
