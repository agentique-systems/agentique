# ConnectionUsage definition-domain authority decision

Decision: implement **AGQ-SYSML20-005**, explicitly selectable SysML Operational
v3. Published, Operational v1 and v2 retain their interpretation. Default
selection remains Published until accepted Systems publication. Enumeration,
variation construction and inheritance fixes are ordinary implementation of
published semantics and are not operational corrections.

## Pinned authority

The unchanged SysML 2.0 PDF is SHA-256
`46e6c0476a6f1f34f367d57e039d56659bff75e41d2e4b3d37ca4cadea84a83a`;
the unchanged SysML.xmi is
`caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`.
The authority is 2.0, not preliminary 2.1.

| Property | Pinned XMI/PDF contract | Query consequence |
| --- | --- | --- |
| Usage::definition | Classifier; redefines Feature::type (8.3.6.4) | Preserve broad classifier typing and reject non-classifier targets. |
| OccurrenceUsage::occurrenceDefinition | Class; redefines Usage::definition (8.3.9.4) | Narrowed domain, not an unconstrained select projection. |
| ItemUsage::itemDefinition | Structure; subsets occurrenceDefinition; deriveItemUsageItemDefinition selects Structure (8.3.10.3) | Exclude valid Class members outside Structure; retain the inherited occurrence domain obligation. |
| PartUsage::partDefinition | PartDefinition; subsets itemDefinition; derivePartUsagePartDefinition selects PartDefinition (8.3.11.3) | Exclude non-PartDefinition Structures; validatePartUsagePartDefinition requires a nonempty result. |
| ConnectionUsage::connectionDefinition | AssociationStructure; subsets itemDefinition and redefines Connector::association (8.3.13.4) | Narrowed association domain, not authority for arbitrary classifier filtering. |
| AttributeUsage::attributeDefinition | DataType; redefines Usage::definition | Non-DataType classifiers remain invalid. |
| PortUsage::portDefinition | PortDefinition; redefines occurrenceDefinition | Non-PortDefinition classifiers remain invalid. |

The existing Item/Part selectByKind implementation was already substantially
correct. The failure was its inherited Class-domain check on the two plain
Association types. [SYSML21-418](https://issues.omg.org/issues/SYSML21-418)
(open, inspected 2026-09-24) corroborates that Item/Part are subsets and have no
additional all-definition-kind restriction. It does not authorize relaxing
OccurrenceUsage's redefinition or ConnectionUsage's association redefinition.

The exact pinned `Flows.sysml` SHA-256 is
`4e65e6a45060d634db8ba16767d7692e7704c62c84b579fdb2e56b18bd0f8a34`.
Its lines 68–69 declare the two anonymous ConnectionUsages typed by
`Occurrences::HappensDuring`, connecting sourceEvent/source and targetEvent/target.
The target is a canonical KerML Association, not Class or AssociationStructure.
Thus source and narrowed published metamodel contracts conflict in the required
corpus. No spelling, imported alias, or missing edge resolves that contradiction.

## Independent reference inspection

Current pilot source was inspected at commit
[`5cca16d846016e62bb1e54e0e50e675254a022ef`](https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/tree/5cca16d846016e62bb1e54e0e50e675254a022ef)
(2026-09-12). Each of the five setting delegates asks FeatureUtil for all types
filtered to its corresponding metaclass. FeatureUtil filters with isInstance.
Exact file URLs and SHA-256 identities are in
[connection-reference-review.json](connection-reference-review.json).

The installed independent pilot 0.59.0 was also run against the unchanged pinned
library and exact Flows source using
`verification/scripts/InspectConnectionDefinitions.java`. It parsed with no
errors and validated with no errors (three unrelated warnings). Both direct
FeatureTyping targets remain HappensDuring. The raw feature-type population is
HappensDuring, Connections::Connection, Objects::BinaryLinkObject. Occurrence,
Item and Connection getters return the latter two; Part returns Connection.

There is a material API distinction: the pilot's runtime getDefinition dispatches
through the redefined occurrence getter and also excludes HappensDuring. Its raw
type population retains it. Agentique's generic current_usage_types deliberately
continues to expose the broad canonical Classifier population. This is an explicit
interpretation, not a claim that all pilot getters have identical behavior.
The installed runtime is not a build of the current source commit; both sources
of evidence are identified independently.

## Narrow correction

Only under Operational v3, when the subject is ConnectionUsage (including its
subtypes) and a broad type is Association but not Class, retain that canonical
typing and broad Usage definition, and exclude it from the occurrence Class
projection. Item, Part and Connection projections then follow their typed subsets.
The filtered target and its canonical metaclass evidence remain inspectable.

All other invalid narrowed-domain targets retain diagnostics, including an
Association on ordinary Occurrence/Item/PartUsage, a DataType on ConnectionUsage,
and incorrect Attribute/Port types. The graph and descriptors are unchanged.
Names never determine applicability. Historical profile manifests are unchanged.
The finalizer, its acceptance criteria, and producer closure requirements are
unchanged. New occurrence/connection effective APIs retain closure evidence.

All four user-authorized operational-decision conditions hold: exact pinned
corpus exercise, contradictory pinned authority, independent reference inspection,
and an explicitly limited correction. The milestone lead approved this boundary.

## Naming and variation distinction

Usage::namingFeature already has an evidence-bearing SysML naming extension,
including VariantMembership reference-subsetting behavior. Removing the blanket
Variation marker from current/effective naming therefore removes a stale marker,
not a semantic check. Effective naming still requires naming producer closure.
Structural queries independently require a closed producer certificate and the
actual canonical specialization to a variant's owning variation Definition.
Unsupported variation-Usage featuring semantics remain pending.
