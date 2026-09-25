# Dependency view: packages are context anchors

The real `real-run03/gallery/07-agent-view.png` shows a ModelRepository dependency
view flooded by siblings reached through its owning PlatformArchitecture package.
The original undirected two-hop walk follows ModelRepository <- owns Package
-> owns unrelated sibling, which is useful generic Graph exploration but an
overly broad dependency neighborhood.

`ViewDefinition.graph_scope` now distinguishes ordinary Neighborhood (the serde
default for existing views) from DependencyNeighborhood. StudioPlatform selects
the latter for its explicit dependency action. The policy is retained with the
view definition rather than inferred from names, so saved views can preserve it.
Only focused Semantic Graph uses this policy; Architecture and Requirements
remain unchanged.

Traversal differs in exactly one direction: a canonical Package reached during
the walk cannot expand its outgoing Ownership relationships unless it was an
original explicit seed. Incoming owners remain reachable context anchors.
Non-Ownership dependencies, including those incident to a package, still expand.
Types and Features retain normal ownership traversal; Namespace would be the
wrong guard because Type is a Namespace. Families, eligible standard IDs,
depth/cap, hidden IDs, canonical edge retention and original ownership are not
changed. This is a bounded neighborhood, not a complete engineering impact query.

Canonical fixture tests cover the package sibling exclusion, part-to-port
ownership, a directly referenced sibling and reference cycle, a package's actual
reference, explicit Package focus, depth zero/one/two/eight, filtered families,
standard eligibility, original edge/owner identities, ordinary Graph behavior,
and serialization/legacy defaults. These tests have not been executed here.
Source-format and diff checks are in `checks/dependency-neighborhood-source.json`.
No runtime consumer, build, language/authority or freshness change occurred.

Integration must retain `graph_scope` alongside depth/hidden IDs when native
`definition()` reprojects the current agent result. A fresh ordinary Graph action
must start with Neighborhood; dismiss/return restores the prior exact scope.
The lead owns that native navigation change and its state tests. The new real
gallery remains pending; the source correction does not claim visual acceptance.
