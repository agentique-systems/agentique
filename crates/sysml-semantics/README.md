# SysML semantic foundation

`agq-sysml-semantics` composes `agq-kerml-semantics` over the same immutable kernel
graph. There is no parser dependency, second model store, copied inherited Usage,
or SysML-specific element identity.

Production contexts require the actual accepted KerML `CompletePublicationOverlay`,
the exact protected snapshot/construction dependency, and an expected
`SysmlDependencyContract`. The compiled acceptance receipt independently establishes
the KerML publication digest, v9 profile, rules, library set, source identity and
descriptor graph. The combined SysML descriptors, SysML profile and rule set,
grammar compatibility and semantic correction manifests, Systems Library identity
and requested standard bindings are also checked. A mismatch is an error. Trusted
descriptor fingerprints are computed once per process and shared by project
contracts; attaching a project does not rebuild the large descriptor registries.
An unpublished Systems candidate never becomes an accepted dependency through this
API.

`SysmlQueries` supplies direct typing, current derived typing, specialization,
reflexive supertypes, owned and inherited usage sets, subsetting, redefinition and
ordinary naming. Attribute/Item/Part projections reuse KerML's algorithms. Usage
typing retains valid KerML Classifiers. A partial-project adapter removes only
proven general ancestors using KerML `all_supertypes` and its full evidence before
checking narrowed domains. Attribute typing requires DataType and Port typing
requires PortDefinition. Item selects Structure from Class typing; Part selects
PartDefinition from the Item subset. Valid excluded types remain in
`filtered_targets`, and out-of-domain candidates remain in `rejected_targets`
with Invalid or pending-typing diagnostics. Port typing and ConnectorAsUsage related
features are structural projections only.

Query results retain the composed KerML evidence. Additional observations retain
canonical origins and proof/search dependencies. The effective usage collection
is an identity set, not normative feature order. Qualified names exclude the root
namespace and retain raw full-name components in namespace order. Duplicate
siblings, pending name populations, and inherited-only/short-name selection stay
explicitly incomplete; the query does not choose by element identity.

Current-graph queries can be complete without claiming SysML producer closure.
Effective queries expose missing Item/Part base relationships, uncomputed derived
`mayTimeVary`, variation/individual implications and the remaining producer-closure
obligation. `SysmlBaselineProfile::Published` retains the final formal singular
`subitem` target. Explicit Operational v1 selects the authorized canonical
`subitems` target and hashes the independent compatibility/correction manifests.
AttributeUsage retains the formal `Base::dataValues` target in both profiles.

`plan_sysml_producers` evaluates metaclass-based base specialization/subsetting
and structural conditional rules one subject at a time. It returns deterministic
canonical relationship proposals and per-rule KerML evidence, including negative
namespace reads. It never changes the accepted KerML publication. The shared
worklist owns application, dirty propagation and closure. Missing formal targets
remain incomplete; there is no source rewriting or alias creation.

`current_usage_may_time_vary` evaluates the exact structural predicate with
canonical Occurrence, SelfLink, HappensLink and Action identities. Incomplete
predicates return no Boolean. `plan_sysml_may_time_vary` is a separate stable-stratum
proposal because later specialization could change this value. The derived property
redefines KerML `isVariable`; the two must not be stored as competing facts.

`StandardSysmlBindings` validates only requested algorithmic anchors, requiring exact
owned paths, unique public declarations, exact metaclasses and source library
provenance. Attribute anchors reuse the accepted KerML bindings. Unbound roles
remain explicit; the bindings API does not bind or validate the whole library.
`with_verified_sources` separately checks original KPAR identity, document/revision,
syntax-node and byte-range evidence for each bound declaration. A candidate binding
set without source verification is not publication acceptance.

Unit tests use an explicitly private synthetic context constructor. Production
textual and programmatic integration must use the actual accepted publication.
