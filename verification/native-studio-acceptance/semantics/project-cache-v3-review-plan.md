# Project semantic cache v3: independent implementation review

**Recommendation: restore the completed source compilation, rather than feed an effective graph back into compilation.** This is a plan, not implemented code or a measured speedup. The first necessary design decision is the authority that can attest an earlier successful validation across process boundaries. Exact hashes supplied by an untrusted cache do not establish that validation occurred.

## Measured target and current call path

The retained `../read-profile-02` evidence reports 181.258 s for authenticated process opening, including 65.722 s runtime publication restoration. Project cache authentication takes 32 ms and compressed decode/frontier hashing 208 ms. Source compilation still takes 108.494 s: preparation 14.702 s, strict kernel validation 620 ms, final closure 37.723 s, final references 7.439 s and effective audit 47.949 s. It rebuilds 1,101 declared records, evaluates 332 producer subjects and audits all 791 local subjects. These are previous observed timings, not results of this review. Nested timing totals must not be added together.

`modeling-service::resolve_revision` checks the durable manifest and loads exact source/checkpoint blobs. Its cache helper calls `ProjectRevisionCheckpoint::restore_cached`, then `SourceIdentityCheckpoint::restore_cached`. That function enters `restore_maybe_cached`, parses source, restores identities and calls `compile_with_history`. The latter still prepares declarations/references, closes producers and audits the final model. `SourceSemanticCache` is first consumed inside final source construction. Ordinary `WorkingProjectRevision::validate` then checks the retained outcomes; it does not itself rerun the audit.

The ZIP v2 format is a codec for the logical v1 cache, not a completed-compilation format. Its kernel frontier already contains local declared records, reservations, effective records, proof/search material and provenance. It does not persist completed closure transport state, final reference answers or the strict effective audit. Replacing ZIP/JSON or duplicating declared records cannot remove the dominant work.

## Trust boundary that must be explicit

`RevisionManifest` and `ValidationReceipt` are public deserializable storage data. `RevisionManifest::verify` performs storage checks. Its documented validation contract requires comparison with rebuilt semantics and checked validation. A caller can manufacture internally consistent cache, receipt and manifest hashes through the public repository interfaces. Adding a public `restore_validated(cache, claimed_digests)` API would therefore widen acceptance authority.

The kernel's bound-frontier reader proves structural/provenance integrity and protected dependency ownership. It does not prove language producer correctness or effective audit completion. Similarly, `ConvergedPublicationFrontier::authenticate` checks exact graph/context/registry, complete certificate coverage, counters, local evaluation rows and transport state without producer replay; it explicitly does not issue acceptance. Its journal digest comes from outside the checkpoint. `ProducerClosureCertificate::from_trusted_receipt` is private because those bytes require independently trusted authority.

The v3 fast path needs a private, authenticated completed-validation attestation minted only from a real `ValidatedProjectRevision`. Two possible contracts are materially different:

- A local validation seal, authenticated with a process/application-owned persistent key outside imported repository/cache data, can preserve the existing untrusted repository-input boundary. It binds the exact cache payload and manifest/source/identity/semantic contracts. Missing seal, lost key or another machine causes source reconstruction and a new seal after real validation. The seal remains discardable acceleration, not source truth. A full OS/key compromise is outside this cache-integrity contract.
- Explicitly trusting a repository implementation to issue validation attestations would avoid that local key, but changes the current public repository contract. A private Rust wrapper alone does not establish trust if arbitrary implementations may return a matching serialized manifest. This option requires an intentional authority contract and adversarial tests, not only a private constructor name.

The local seal is the narrower trust expansion for a desktop cache. This review does not authorize a specific key-store implementation or claim one already exists. Do not block independent producer/read optimizations on that decision, and do not skip audit while it remains unresolved.

## Minimal payload and restoration sequence

Use a separately versioned v3 envelope. Preserve v1/v2 decoding and source fallback; no migration of authoritative sources or accepted publication identities is necessary.

1. Authenticate the immutable manifest, source bytes and identity checkpoint as today. Require the exact durable revision/project/parent, source binding, accepted KerML and SysML publication identities, registry/descriptor/rule/profile contracts, and explicit cache/audit implementation versions. Bind the seal to the payload digest, manifest binding and all of those identities. Working revisions never take this validated fast path. A model digest or a source SHA alone is insufficient.
2. Reuse the existing bounded source parser/identity restoration to produce editable `SourceInputs`; do not lower or refine references. This authenticates document and syntax identity populations, byte digests, limits and source status. Restore declared reservation/retirement history and source ledger with their existing duplicate and lineage checks. AST persistence is unnecessary for the first implementation.
3. Decode the already-present bound local frontier directly over the independently authenticated immutable dependency. Add a kernel reader variant that accepts that dependency, registry and bound dependency identity without requiring a freshly lowered snapshot. It must retain ordinary kernel validation: local/protected identity separation, exact reservations, record/slot/link shape, declared origins, proof dependency/cycle checks, contribution/search/navigation integrity and strict obligations. The result remains an unaccepted kernel overlay. Its actual recomputed graph digest must match the authenticated attestation; cache-supplied digest labels are not enough.
4. Restore the complete producer certificate and interned transport reads, plus status/evaluation-row coverage, using the same invariants as converged publication frontier authentication. Require exact graph, registry, semantic context contract and complete local population. Do not globally schedule producers or rebind a certificate from a different semantic model. Recompute counters from the restored state rather than trusting reporting labels.
5. Restore the final source map and reference assertions, each under the authenticated payload. Reference target IDs and evidence must refer to the restored graph; source origins must bind to the parsed source identities. Serialize query evidence deliberately rather than replacing reference answers with only target IDs. Unsupported reference/evidence variants are a cache miss. Fresh runtime revision labels require explicit, checked rebinding after semantic context equality; never return evidence from the previous in-memory context.
6. Restore the completed strict audit under the same attestation: audit implementation contract, exact ordered local subject population, complete report/family counts and zero findings. Construct its fresh SysML context from the restored model and accepted dependency, then require semantic context equality. Only a narrow private attested path may construct this completed audit. A generic Serde constructor for `SourceEffectiveAudit` would bypass its present construction invariant.
7. Assemble `SourceCompilation` and run the inexpensive workspace acceptance checks. Retain the existing query contexts and revision-bound read/projection caches. Initially set audit incremental-reuse state and lowering fragments to absent unless their full read/signature/identity evidence is persisted and authenticated. That safely permits warm reads; it may make the first subsequent edit colder, which must be measured and reported. Do not manufacture reused-query counts from a restored audit report.

`SourceCompilation` also carries declared history, identity ledger and edit-frontier bookkeeping. Restore their exact authenticated state or reconstruct the neutral same-revision state explicitly; do not silently substitute unrelated empty histories. Producer transport reads are worth keeping in the initial payload because they are required for sound later invalidation. Persistence of the new audit-reuse receipts can follow once their complete format and context rebinding are proven.

## Concrete implementation boundaries

| Area | Bounded change |
| --- | --- |
| `modeling-service/src/cache_codec.rs`, `lib.rs` | v3 envelope, outer exact binding/attestation check, source fallback, distinct timing/work counters |
| `modeling-workspace/src/checkpoint.rs` | export only from validated handle; narrow completed-restoration boundary; no untrusted public acceptance constructor |
| `kerml-text/src/source_semantic_cache.rs`, `source_checkpoint.rs`, `source_inputs.rs`, `sysml/source.rs` | completed local payload, parse/identity helper shared with cold restore, source-model and audit restoration after attestation |
| `kernel/src/archive.rs` | structurally checked bound-frontier decoder over an independently supplied dependency; never language acceptance |
| `kerml-semantics/src/producer_closure_frontier.rs`, `publication_frontier.rs` | factor existing certificate/row/read integrity checks for the attested project path without exposing arbitrary accepted-receipt restoration |
| repository/application host boundary | chosen persistent validation-attestation authority; independent of cache-supplied hashes |

Do not add persisted global indexes first. The current decoder shares immutable dependency indexes and builds local declared/effective indexes; it is not blindly rebuilding all 76k standard records for each project. Measure kernel decode, index build, model/context hashing, certificate authentication, reference evidence decode and retained query-context setup separately. Additional indexes may be persisted only if those measured costs remain dominant and their exact registry/model authentication is specified.

## Required proof before using the path

- Separate-process v3 restoration equals cold reconstruction for canonical graph, occurrences, identities, derived facts, complete query values/evidence/searches, closure semantic digest, references and validation diagnostics.
- Changing source, syntax/reservation history, publication, profile, rules, producer registry, audit implementation or revision binding rejects the cache. Recomputing all cache-local hashes must not turn an unattested fabricated audit into Validated.
- Missing/extra/duplicate audit subjects, incomplete family rows, altered certificate/read state, wrong-context reference evidence and attempted writes to protected dependency IDs reject the cache.
- Truncated/oversized/duplicate-entry archives, missing seal and lost key fall back without mutating durable head. No half-restored revision enters the shared read cache.
- Delete all disposable caches/attestations and reconstruct source-only in a separate process with identical required semantic identities and answers.
- Create/rename/cancel/validate/commit from a v3-restored revision, then restart. Exact parent binding and full cold oracle remain required; first-edit latency is reported separately from warm reads.
- Successful restoration reports actual producer and audit executions as zero, with restored populations in separate counters. Kernel integrity checks, graph/context hashing and attestation verification remain inside the measured open boundary.

This path targets the roughly 108 s authored compilation cost, but no speed estimate follows from source inspection. Even a very fast project v3 restore cannot deliver <10 s full process launch while authenticated runtime restoration remains 65.722 s. That separate boundary needs its own profiling and exact accepted-publication identity checks; it must not be hidden outside the final startup measurement.

Review performed read-only for cache production code by Codex `/root/worlds`. No cache API, cache authority, repository receipt or acceptance pin changed in this task.
