# ADR 0027: compositional semantic publication

Status: proposed research architecture; not adopted and not current publication
authority. Written before implementation on `068f7b0`.

The immediate Systems acceptance path is the existing monolithic dependency-driven
scheduler, including its Structural and ContextualBindings strata. Its complete
fixed point and fully closed certificate remain mandatory. ADR 0027 does not block
that acceptance or adoption of ADR 0026.

The corpus experiment established that current read-side provider/search
footprints are insufficient to prove useful strata. It did not establish that
Systems is semantically indivisible. Retain the component planner, exact semantic
comparator, boundary audits and invalid-partition tests as research infrastructure.
No further Systems partition planning is required by this milestone. Precise
provider footprints can later support authored ProjectWorkspace recomputation;
the initial workspace does not depend on unproved component sealing.

## Decision and authority

Canonical publication establishes semantic closure, not a particular scheduler
invocation. Extend [ADR 0025](0025-producer-closure-evidence.md) with independently
sealed semantic components and a hierarchical certificate. Keep its existing
producer registry, potential effects, provider searches and private certificate
issuing boundary. A dependency plan alone conveys no closure or acceptance.

Inputs remain pinned KerML 1.0 and SysML 2.0 with explicit profiles. Accepted
KerML Operational v9 stays immutable and accepted. Published and SysML
Operational v1/v2 remain distinct. Gen1 obligations are unchanged.

## Equivalence contract

Let I contain the complete declared source graph and its provenance, profiles,
producer registry, rule identities, bindings and accepted immutable publications.
Let Close be the existing deterministic producer fixed point, including its
structural/contextual phase boundary, query evidence and failure semantics.
For a certified partition P of the local declared subjects, publication requires:

```text
Compose(Close(P1, I), ..., Close(Pn, I)) == Close(union(P), I)
```

Equality covers canonical Elements, slots, ordered ownership, association
occurrences, derived identities, canonical proof and search contents, effective
query values/completeness/evidence, closed semantic requirements and semantic
graph digest. A root composition certificate has its own versioned identity;
its shape need not equal a flat scheduler certificate. No equality is inferred
from counts, convergence, successful parsing or kernel validity alone.

The sufficient boundary conditions are:

1. All local declared subjects belong to exactly one component; accepted standard
   subjects remain authenticated external providers. Generated subjects belong to
   the component of their creating evaluation. Conflicting creation identities,
   unresolved carriers or unknown attachment scope prevent sealing.
2. A component runs the existing producer families to a local fixed point,
   including newly created applicable subjects and every phase. It must close
   every applicable pair and every semantic requirement needed by its outputs,
   references and certificate. Incomplete or unsupported semantics stay explicit.
3. Every observation affecting applicability, output selection, facts, proofs or
   searches sees the same population as in whole-graph closure. Its providers are
   fixed declared facts, local producers, predecessor components, or accepted
   immutable publications. Absence requires the same potential-writer proof as
   positive facts. Current read values alone cannot establish this condition.
4. No later component, including any family activated on a future generated
   subject, can change a fact, effect or search population required by the sealed
   certificate. Unknown scope conservatively joins the affected populations or
   prevents sealing. A source import graph is not this proof.
5. Additive outputs commute across independent components, including ordered
   contributions and proof/search contents. A selected output that depends on a
   later population violates condition 3 even if it has a stable derived ID.

Under these conditions, induction over a topological order establishes equality:
predecessor results are equal; all observations relevant to the next component
are fixed and equal; its unchanged local fixed-point evaluator therefore emits
the same facts and evidence; subsequent components cannot invalidate them.
Independent adjacent components commute by conditions 3–5, so legal order
permutations preserve the result. Cyclic dependencies require joint closure.
This conditional argument is not a claim that the current registry proves a
useful Systems partition. Missing evidence must fail closed.

## Semantic dependency and writer graph

Vertices start at declared Systems subjects, not documents. Dependency edges
point from a consumer to a provider. Include imports, mandatory reference and
standard-target resolution, ownership, typing, specialization, subsetting,
redefinition, positive proof reads and negative/provider searches. Include
potential writers to required effects even when their current output is empty.

Reuse `ProducerDescriptor`, `ProducerFamily`, `ProducerEffect`, applicability,
per-effect scope, fresh-subject effects and attachment contracts. There is no
second effect registry. Preserve reasons for edges and conservative fallback.
Strongly connected components form a condensation DAG; providers close before
consumers. An unknown or model-wide writer may legitimately join the entire
local population. The planner must report that result rather than split it.

A later subject may type against, specialize, subset, reference or import an
earlier subject by creating a relationship whose canonical carrier belongs to
the later component. That is not permission to mutate the earlier carrier or
its ownership. Incoming and inverse searches can nevertheless observe such a
relationship: if an earlier certificate requires that population, its dependency
on the later writer forces joint closure. Both directions must be accounted for.

## Sealing and certificate composition

A sealed stratum contains its shared graph delta, subject membership, dependency
summary, mandatory reference results and scheduler-issued component certificate.
Its certificate binds the exact subject set and graph content, dependency
certificate digests, registry, interpretation contract, closed requirements,
provider/search evidence and future-writer exclusion proof. Public callers cannot
construct completeness from a list of IDs or a digest.

The composite certificate binds the canonical ordered DAG identity, component
certificate digests, complete semantic graph digest, registry and context
contract. Ordering uses semantic identities, not execution order or addresses.
Explain traces a subject/requirement to its component, closed producer effects,
authenticated external providers and exclusion of downstream writers. Preserve
`AcceptedDependency` as a distinct explanation boundary.

Trusted restoration authenticates the entire tree against independently expected
source, registry, context and publication authority. Rehashing caller-provided
Complete labels is insufficient. Structural cache loading alone confers no
acceptance. Restoring an accepted tree does not replay component producers.

## Storage, checkpoints and invalidation

Queries see one logical canonical graph: accepted KerML plus sealed deltas plus
the current open component. Share immutable records and proofs; do not flatten
or copy accepted standard records into a new semantic universe. Release sealed
evaluation rows, temporary causal indexes and planning allocations. Retain the
canonical delta, explanation/search data, compact certificate and dependency
summary needed to verify the boundary.

Checkpoints bind source commit and bytes, profiles, producer registry, dependency
certificates, component graph and certificate digests. Write a complete candidate
before durable replacement. Verify all identities and tree boundaries before
reuse; partial files or changed inputs are rejected. Checkpoints live in ignored
generated storage and are not accepted-publication authority.

An edit or violated dependency contract invalidates the affected component and
its dependent DAG closure explicitly. A sealed component is not periodically
replayed while later components run. Full authored incremental recomputation is
outside this decision's initial implementation.

## Future compositional acceptance sequence

First test synthetic linear, diamond, cyclic, typing, subsetting, negative-search,
late-writer, relationship-carrier, variable-feature and feature-chain cases against
both existing full-scan and worklist evaluators. Deliberate unsound splits must
fail. Legal topological order permutations must give exact semantic equality.

Then compare the existing 13-document candidate under monolithic and
compositional closure with exact content/evidence equality. Only a passing result
authorizes component-by-component full Systems closure. Report the real subject
DAG, document membership, pairs, references and read/write boundaries.

After every Systems component seals, perform one global read-only acceptance
audit: 21/21 exact operational parses, zero kernel obligations, all 1,327 distinct
mandatory references Complete with matching endpoints, valid component/root
certificates, zero authority/capability findings and clean identity/provenance.
Do not reconstruct the monolithic scheduler for that audit.

Only that result accepts `CanonicalSysmlSystemsLibrary`, accepted bindings and a
trusted receipt/cache. Adoption of ADR 0026, real Agentique self-model acceptance
and effective SysML API acceptance follow. Their readiness decision authorizes
the additive Gen2 modeling workspace. Full SysML conformance is not another gate.
If equivalence cannot be established, preserve the publication gate and record
the exact failed boundary. No production workspace integration is authorized by
this proposed ADR alone.
