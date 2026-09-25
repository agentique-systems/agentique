# Dependency neighborhood: independent source review

Reviewed exact commit `d9a626f2844f84e4613d76e39677f72cf907f5d0` after the real
ModelRepository dependency gallery exposed package-mediated sibling expansion.
**Source judgment: pass, conditional on integrated tests and new real images.**

`StudioPlatform::dependencies` now selects an explicit
`GraphScope::DependencyNeighborhood`. Its serde-default field preserves older
saved views as ordinary Neighborhood and lets a saved dependency view retain its
scope. Architecture, Requirements and an unfocused Graph ignore the new policy.
The scope is described as a bounded neighborhood, not a complete impact analysis.

The traversal changes one direction of one family: outgoing Ownership from a
canonical Package is terminal unless that Package was the original seed.
Incoming ownership still adds the real package as context. Type/Feature ownership
and all non-Ownership relationships still expand. The code checks Package rather
than Namespace, avoiding the mistake of suppressing every Type's children.
Existing relationship-family and standard-identity filters supply the same
eligible nodes and edges. Final edge retention preserves the original canonical
edge objects, relationship IDs, endpoints, provenance and evidence; no dependency
shortcut, copied feature or invented owner is introduced.

The source tests exercise a Package with unrelated siblings, a Type-owned port,
a directly referenced sibling, a reference cycle and a genuine package reference.
They contrast ordinary Graph with dependency scope, cover depth zero/one/two/eight,
explicit package focus, filtered families, excluded/explicitly focused standard
identities and serialization defaults/round trips. They assert original owner
identity and unchanged edge data. These tests were inspected, not run by this
reviewer.

No source blocker was found. Native integration must still distinguish deliberate
reprojection of an agent result from opening an ordinary Graph: preserve the
dependency scope for the former, select Neighborhood for a fresh ordinary Graph,
and restore the previous exact scope when dismissing the agent view. A new real
gallery must show whether removing package siblings improves label and edge
readability; fewer records alone do not establish product quality.
