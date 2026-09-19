# Operational KerML profiles

Published KerML 1.0 (`omg-kerml-1.0-published/1`) is the exact formal baseline.
Operational KerML 1.0/v1 (`agentique-kerml-1.0-operational/1`) adds only
AGQ-KERML10-001 / KERML11-81. Its manifest and identity remain frozen.
Operational KerML 1.0/v2 (`agentique-kerml-1.0-operational/2`) additionally
applies AGQ-KERML10-002 / KERML11-140. This is an Agentique semantic
interpretation authorized by the project, not an adopted OMG erratum.
KERML11-140 remains open, last updated 2025-11-11 23:25 GMT, as captured
2026-09-19. It does not apply to any future KerML version.

## Reviewed resolution algorithm, version 1

The entry point is exclusively `Redefinition::redefinedFeature`, for an owned
redefinition whose owning feature has an owning type. Ordinary names, typing,
subsetting, and ordinary qualified-name operations retain their existing rules.

1. Explicit global qualification uses basic resolution from the declared root
   availability. It does not try inherited or lexical alternatives.
2. For a relative name, search the first segment independently in every
   general type of the specific owning type, excluding the redefining declaration.
   Each general namespace applies its ordinary inherited redefinition suppression.
   Collect all matching memberships across the general scopes, then remove a
   matching candidate only when another matching candidate redefines it. A
   differently named redefinition on another branch does not suppress a match
   independently reachable through this branch. Retain canonical membership identity. Do not search lexical parents of the general types.
3. Only a complete empty first-segment search permits searching the lexical
   namespace containing the specific owning type, then its actual containing
   namespaces using basic resolution. The redefining declaration is excluded
   throughout membership computation, including its suppression of inherited
   members. There is no search of unrelated namespaces.
4. If any first segment is found, resolve remaining segments through ordinary
   public visible membership lookup. Prefix shadowing, wrong kind, ambiguity,
   incomplete evidence, and a missing qualified suffix never trigger another
   scope search. Inherited members are non-private; private members are visible
   only through normal internal lexical access. Imports and aliases retain their
   normal visibility and identity contracts.
5. A target must be a Feature distinct from the redefining feature. Name
   denotation does not waive separate redefinition conformance obligations.
   Multiple incomparable memberships remain ambiguous. No traversal order picks
   a winner. Positive and search dependencies include the relationship, searched
   scopes, visibility, and the explicit operational rule path and profile.

This refines the proposed "published search then lexical" wording using the
actual reference implementation: `KerMLScope.resolve` begins with `gen`, which
searches inherited members without general lexical parents; its parent scopes
come from `KerMLScopeProvider.scopeFor` on the specific namespace. All
specializations participate; matched same-name redefinitions suppress their targets. Agentique does
not adopt the implementation's `findFirst` selection of incomparable candidates.

## Evidence boundaries

The [machine-readable authority packet](../verification/kerml-name-resolution-errata-publication-v4/authority-packet.json)
pins original issue HTML, formal and preliminary PDF clauses, reference source,
tests, release notes, and generated XMI. The reference repository commit is
`5cca16d846016e62bb1e54e0e50e675254a022ef`; the release repository commit is
`fb97b754f29588b8e9c7a35f370880cd15eb29e7`.

The two nested `monitoredFeature` redefinitions both point to XMI element
`6b06695b-e0d3-5d4e-aca5-af32f4ad40d8`, the enclosing monitored feature. Their
source patterns are unchanged in the pinned library. Current reference library
files also contain unrelated revisions; the packet records their exact byte
identities and source differences. They are not substitutes for the pinned
source or normative authority.

Publication acceptance remains subject to the full structural obligation,
validation, binding, and authored-project gates. This profile definition alone
does not establish accepted library publication or SysML readiness.
