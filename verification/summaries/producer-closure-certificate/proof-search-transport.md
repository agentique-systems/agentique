# Producer proof search transport

Canonical kernel searches preserve three previously conflated dependencies:

- `SourceRelationships { source, class, property }` retains the exact
  descriptor-resolved source role, including subtype/property redefinitions.
  An unrelated relationship general endpoint is not part of that population.
- `ElementIdentity` reads only existence/metaclass; `Element` continues to cover
  arbitrary stored properties and populations. Missing identities stay open.
- `ProducerClosure { subject, requirement }` is a reopenable semantic dependency.
  The requirement is a versioned opaque language contract, not a model Element.

The kernel closure search deliberately contains no certificate digest. Exact
current certificates remain shared query-context sidecars. Language
`SearchDependency::ProducerClosure` carries the attached digest for query-cache
identity and Explain evidence. A query needing the negative premise again must
obtain `producer_closure(subject, requirement)` from its current context; a stored
kernel search never supplies a successful witness by itself.

This split prevents intermediate frontier/worklist certificate history from
changing the canonical graph or accumulating distinct canonical searches. The
regression builds equivalent derived overlays from query evidence with different
certificate digests and with repeated/missing historical witnesses: transported
search populations and semantic graph digests are identical, while the original
query evidence remains distinct. Ordinary revision invalidation remains global
for a closure boundary and bounded by the source for source-role searches.

Existing canonical digest tags and archive encodings are unchanged. New search
variants append tags 6/7/8; accepted KerML Operational v9 restoration stays pinned.
Archive roundtrip retains typed searches, proof sharing and deterministic bytes.

Verification uses the isolated `target/foundation-evidence` directory with
incremental/debug artifacts disabled and two build jobs. Completed commands:

- `cargo test --locked --offline -p agq-kerml-semantics --lib -p agq-kernel --test archive`:
  exit 0, 70 semantic unit tests, 10 kernel unit tests, 5 archive tests;
  two existing semantic scale probes remain ignored.
- `cargo test --locked --offline -p agq-kerml-semantics --test relationship_population --test foundation`:
  exit 0, 4 population tests and 29 foundation tests.
- After separating certificates from canonical searches:
  `cargo test --locked --offline -p agq-kerml-semantics --lib transported_closure_requirements`
  and `cargo test --locked --offline -p agq-kerml-semantics --lib closure_witness_is_invalidated`:
  exit 0, one focused test each; kernel archive suite rerun, exit 0, five tests.

Development fixture correction: adding a global certificate dependency to an
existing explicitly bounded-read fixture correctly failed its non-global
assertion. Certificate transport now has a separate global-invalidation test;
the bounded fixture continues to test only bounded reads.
