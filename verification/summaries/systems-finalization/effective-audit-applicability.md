# Effective audit applicability and typed-domain diagnosis

Status: **acceptance remains blocked**. This is a read-only investigation of the
failed finalization, not a new publication or a producer run. No source bytes,
canonical records, query gates, producer rules, or checkpoint identities changed.

## Scope of the original acceptance gate

`crates/kerml-text/src/sysml/publication.rs` is byte-identical at original scheduler
source `7223938437dc8ab7c684961bfb76eaba6c6c92f0` and inspected main
`0c4c0adae19125c5a72b3e3521c7df1c6c6d84f0`. Its `audit_sysml_population`
already audited effective names, current effective usages, direct specializations,
current usage types, current attribute/item/part/port definitions, and current
connection related features. The finalizer retains those checks and additionally
audits the effective API population required by the milestone.

The retained failed report contains these failures of operations already present
in that original gate:

| Operation | Failed subject/operation pairs | Family-weighted outcome findings |
| --- | ---: | ---: |
| effective names | 21 | 63 |
| current item definitions | 2 | 8 |
| current part definitions | 2 | 8 |
| Total | 25 | 79 |

The 79 count is specifically `SQ_PUBLICATION_TYPED_QUERY` outcome records.
Underlying diagnostics are emitted separately; it is not a total for an imagined
rerun of the historical audit. Thus removing newly added effective checks would
still leave acceptance failures.

The new return-parameter audit calls `effective_return_parameters` on its existing
Definition/Usage domain, as implemented in
`crates/sysml-semantics/src/structural_queries.rs`. The narrower Function/Expression
domain belongs to the different KerML `structural_result` query. Restricting the
generic return query to actions or functions would change its acceptance coverage.
The reported cycle findings require their own query diagnosis.

## Two explicit connection typings fail the existing domain contract

The retained Systems graph contains these declared facts:

| ConnectionUsage | FeatureTyping | Target |
| --- | --- | --- |
| `8e5fc3a7-4224-5f80-8c41-801c3bb1b6ca` | `81be881d-6fc1-57d1-8434-a7267d640b15` | `51a76600-2466-5695-8c53-8e327f6302de` |
| `e76d30a1-2f36-584d-8dce-409db017b746` | `48dd041c-6912-599a-8a2a-93faab66570a` | `51a76600-2466-5695-8c53-8e327f6302de` |

The immutable accepted KerML cache identifies the target as
`Occurrences::HappensDuring`, metaclass **Association**. This agrees with
`standards/libraries/Semantic-Library/Kernel Semantic Library/Occurrences.kerml:745`,
which declares `assoc all HappensDuring`. The two Systems declarations are the
`connection :HappensDuring connect ...` statements in
`standards/libraries/Systems-Library/Systems Library/Flows.sysml:68` and `:69`.

Pinned `standards/normative/sysml-2.0/SysML.xmi` establishes:

- `Usage::definition` redefines KerML `Feature::type` with Classifier domain.
- `OccurrenceUsage::occurrenceDefinition` redefines `Usage::definition` with
  **Class** domain.
- `ItemUsage::itemDefinition` selects **Structure** from occurrence definitions.
- `PartUsage::partDefinition` selects **PartDefinition** from item definitions.
- `ConnectionUsage` inherits PartUsage; its `connectionDefinition` has
  **AssociationStructure** domain and subsets `itemDefinition`.

An Association is not a Class or AssociationStructure. The existing
`current_item_definitions`/`current_part_definitions` rejection therefore cannot
be repaired by exempting ConnectionUsage, filtering the explicit incompatible
type, or treating Association as Class. The exact retained graph preserves a
pinned library/metamodel domain incompatibility. Resolving that incompatibility
requires an explicit authority-backed language-maintenance decision; this review
does not change the pinned library, accepted KerML, or the checkpoint.

## Interface end rejection exposes a separate query-composition defect

`Interfaces::binaryInterfaces` is InterfaceUsage
`8ce7e6db-fd66-5ba1-9cd9-a09a77092859`. Its graph has no owned ends. It is typed by
`Interfaces::BinaryInterface` and subsets both `interfaces` and
`Connections::binaryConnections`. Those paths bring in these end identities:

| Position | BinaryInterface PortUsage | BinaryConnection ReferenceUsage | BinaryLink Feature |
| --- | --- | --- | --- |
| source | `45511382-ea84-5db3-b172-c1acd4acd229` | `6c7986f9-6e5b-59f0-83e6-51e1fb686d81` | `cdcb027b-1d54-52bb-be1f-f48f1a8361f1` |
| target | `1e65a6e7-843d-5965-b645-2aeb545bbb03` | `66864d33-a47d-5da9-9da6-41b7aa317ee1` | `4167e261-a897-5f30-a4cb-0bc37f94b0ec` |

For each row, explicit canonical Redefinition relationships already connect the
PortUsage to the ReferenceUsage and that ReferenceUsage to the Feature. These
agree with the declarations in `Interfaces.sysml` and `Connections.sysml`.
The query nevertheless returns the two generic BinaryLink Features as well,
causing `SQ_ELEMENT_KIND` in `effective_interface_ends`.

`KerMlQueries::structural_positioned_features` in
`crates/kerml-semantics/src/implicit.rs` computes transitive explicit suppression
from **owned** positioned features. At `binaryInterfaces`, the owned set is empty;
redefinitions carried by inherited ends from one general do not suppress the
generic ends arriving through another general. This is an immutable-query
composition defect, not missing canonical redefinition facts.

A sound repair must account for transitive redefinitions between inherited end
candidates, preserve semantic order and canonical identities, and retain the
supporting relationship/search evidence. It must not relax the PortUsage check
or copy/mutate inherited records. That repair remains unimplemented here; it
does not resolve the independent connection-domain or enumeration findings.

## Evidence and verification

Read-only input archives:

- Original retained Systems frontier
  `verification/generated/final-language-acceptance/full-frontiers/c42d679d2908df7ba0a864710eda0e4e8c8cb97e70ead2ef5e9c604888bd447d.zip`.
- Original accepted KerML cache
  `verification/generated/kerml-v9-publication/canonical.publication.zip`.
- Failed finalizer report under
  `verification/generated/systems-finalization/.accepted.candidate-20172988-64ac-4178-9654-be4bb021a245/report.json`.

Commands completed in the isolated `platform/typed-domain-audit-fix` worktree:

| Command | Result | Exit |
| --- | --- | ---: |
| `git show <revision>:crates/kerml-text/src/sysml/publication.rs` for both source revisions, compared using Python | historical files identical | 0 |
| `python -` using `zipfile`, `json`, and generated typed-view UUID constants | streamed both archives; decoded 75,342 distinct canonical records; confirmed the listed typing and redefinition identities | 0 |
| `python -` using `xml.etree.ElementTree` | inspected the pinned XMI property domains, redefinitions, and subset constraints above | 0 |
| `python -` over the failed report | original-operation outcome counts 63 + 8 + 8; all-report diagnostics 592 typed-query outcomes, 40 element-kind, 33 result-inheritance-cycle, 9 end-cycle | 0 |

Small decoded extracts remain under ignored
`verification/generated/typed-domain-investigation/` in the investigation
worktree. No Rust build, finalizer rerun, or producer execution was performed.
