# Pinned Objects library authority conflict: KNRV4-F-001

Strict semantic publication of the currently pinned KerML libraries is blocked
by indistinguishable inherited features in `Objects.kerml`. This finding is
separate from KERML11-140. No additional operational correction is applied.

The pinned source is
`standards/libraries/Semantic-Library/Kernel Semantic Library/Objects.kerml`,
SHA-256 `2b2b33dc326bd188cb8d2400aaf73cfa2546baf9d3872cfe32aa2aa51b512e0f`.
The source carried by the official reference release at commit
`e0ccd90b5567f873f99ec6afe62e3502a9f63c47` is byte-for-byte identical.
Its generated XMI supplies resolved relationship endpoints independently of
Agentique's parser, IDs and resolver. XMI corroborates an implementation; it is
not normative authority.

| Specific feature | Explicit typing | Explicit subsetting | Surviving same-name features |
| --- | --- | --- | --- |
| `StructuredSpaceObject::faces` | `Surface` | `structuredSpaceObjectCells` | `Surface::innerSpaceDimension`, `StructuredSpaceObject::innerSpaceDimension` |
| `StructuredSpaceObject::edges` | `Curve` | `structuredSpaceObjectCells` | `Curve::innerSpaceDimension`, `StructuredSpaceObject::innerSpaceDimension` |
| `StructuredSpaceObject::vertices` | `Point` | `structuredSpaceObjectCells` | `Point::innerSpaceDimension`, `StructuredSpaceObject::innerSpaceDimension` |

`structuredSpaceObjectCells` is typed by `StructuredSpaceObject`. Each pair
contains distinct, public, undirected, non-end Features. Both redefine the same
`Occurrences::Occurrence::innerSpaceDimension`; neither redefines the other.
The specific features have no owned feature that redefines either pair.
KerML 1.0 `Type::removeRedefinedFeatures` therefore retains both memberships.
Their identical effective names and identical metaclass violate
`validateNamespaceDistinguishibility` (the spelling in the formal metamodel).
No positional end or parameter rule supplies a common redefinition here.

The [independent authority packet](../verification/kerml-name-resolution-errata-publication-v4/separate-authority/authority-packet.json)
contains the actual XML, relationship and membership IDs, exact artifact
hashes, formal constraint and operation bodies, and three checked witnesses.
The [offline verifier](../verification/kerml-name-resolution-errata-publication-v4/objects-authority.py)
reads those original XMI files directly. Its suppression calculation is a
projection onto these specific conflicting features, not a claim to implement
all KerML semantics independently. A separate
[synthetic canonical witness](../crates/kerml-semantics/tests/library_authority_conflict.rs)
reproduces the conflict under published, operational v1 and operational v2
profiles. Explicitly adding a common redefinition fixes that synthetic witness.
That repair is not inserted into the pinned library.

Current reference source and XMI at commit
`fb97b754f29588b8e9c7a35f370880cd15eb29e7` explicitly introduce nested
`StructuredSurface`, `StructuredCurve` and `StructuredPoint`. Each has a local
feature redefining both conflicting features; the three affected features are
typed by those combined types. Source comments describe the redefinitions as
necessary. The exact [source diff](../verification/kerml-name-resolution-errata-publication-v4/separate-authority/Objects-pinned-vs-current.diff)
also retains other changes, rather than implying the new file is interchangeable
with the pinned one. The repair already appears in the 2025-07 reference
release. The separately preserved `standards/libraries-2026-04` artifact also
contains revised source; it has a distinct content identity and is not the
library set under test.

[KERML11-72](https://issues.omg.org/issues/KERML11-72), captured open with update
date 2025-07-09 02:55 GMT, discusses the related general multiple-inheritance
anomaly. It is not represented here as a formally adopted correction or as an
issue filed for these exact Objects declarations. The authority conflict rests
on the pinned source and the published constraints, independently of that issue.

The v4 stop follows the user's separate-authority-conflict policy. Acceptance
would require a separately reviewed, versioned correction to the pinned library
source interpretation, or an explicitly selected replacement library artifact
with its own identities and verification. KERML11-140 authorizes neither action.
Choosing a sibling by traversal order, weakening distinguishability, inventing
common redefinitions, or silently switching library bytes would conceal the
conflict. No accepted library facade or accepted bindings are published.
