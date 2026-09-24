# In-memory Gen2 modeling workspace

`ProjectWorkspace` owns one Working head and immutable revision history over
authenticated accepted KerML and Systems publications. The measured
[language readiness decision](../../verification/summaries/final-language-acceptance/semantic-closure-readiness.md)
adopts ADR 0026 and authorizes this integration. The
[runtime record](../../verification/summaries/final-audit-semantic-closure/workspace-runtime-acceptance.md)
establishes all 12 unique accepted-cache test passes: workspace self-model,
five Working-state tests, four edit/recovery lifecycle tests, five Validated
revisions with 100 mixed documents each and four parallel readers, and the
separate recovery-scale sequence with a Working fourth revision and repaired
fifth revision. ADR 0024 is adopted for this bounded in-memory Phase 1 contract.

Document operations check an expected head, prepare exact source/syntax inputs,
compile the current document set, then publish one new revision atomically.
Stale heads, invalid edit spans, missing documents and construction invariant
failures publish nothing. Recovery, unresolved references and explicitly
unsupported source become Working revisions with native diagnostics. Their
queries borrow the current construction/overlay and preserve incomplete scope
evidence. They never fall back to an earlier revision's graph.

`WorkingProjectRevision::validate` returns a `ValidatedProjectRevision` for the
same immutable handle only after the Phase1V1 contract passes: complete parsed
inputs, a strict declared graph, no blocking diagnostics, Complete converged
producer closure, a fully closed certificate attached to the exact context,
complete mandatory references, and a finding-free effective-query audit bound to
that revision. The audit reuses the strict Systems effective dispatcher on every
local canonical identity, including derived local records, in bounded batches.
It excludes the shared accepted dependency population. Validation does not claim full language
conformance or execution support.

Both handles expose the existing KerML and SysML query contracts. Element/source
lookup and diagnostics are revision-bound; there is no second semantic DTO
model. Accepted standards remain shared, immutable dependencies. Authored
construction still passes full kernel validation for each edit. Unchanged
documents share immutable syntax and pre-resolution lowering fragments. When
the prior authored frontier is strict, construction submits only the new,
changed and removed declared records to an ordinary kernel transaction;
unchanged records retain their shared allocations. A reference-refinement pass
still clears any old endpoint that its current declarations have not resolved.
Incomplete predecessors use full reconstruction. Global
structural completion and reference refinement run on the assembled current
declarations; native producer checkpoints revalidate query/provider reads before
retaining an evaluation. Broad namespace/search dependencies reopen work
conservatively. Every final local subject is still effectively audited.

Repository callers use `prepare` to construct a detached candidate and call
`acknowledge` only after their durable transaction commits. `from_revision` opens
an independent branch workspace without copying graphs. `apply` remains the
Phase 1 in-memory convenience operation.

`ProjectRevisionCheckpoint` is an explicit versioned identity carrier. Source
bytes are supplied separately by DocumentId and checked against SHA-256 digests.
Restore reparses those bytes, checks every syntax-node shape before restoring
its identity, restores active/retired reservations against the authenticated
standards, and reconstructs semantics through the ordinary language engine.
Only a Working handle is returned. A repository must authenticate its validation
receipt and call the actual platform validation conversion independently.
Kernel revision labels are freshly allocated; canonical graph/provenance,
semantic-contract and closure fingerprints are independently comparable.

Validated revisions can additionally export `ProjectSemanticCache`. Its local
kernel frontier excludes accepted standard records and source bytes. Restore
checks the exact source-checkpoint digest, cache checksum, language/descriptor/
producer dependencies, canonical graph and closure identities. It reconstructs
source declarations, revalidates cached effective facts against them, and reruns
the producer scheduler and every effective-audit subject. It returns Working;
the service independently verifies its persisted receipt before validation.
Any cache failure permits ordinary source reconstruction. This conservative
cache reuses persisted effective facts while retaining source parsing and audit
work; it is disposable and supplies no independent semantic authority.

`compilation_work` separates documents reparsed/lowered, lowering cache hits,
records lowered, declared records changed/removed versus retained across
construction passes, producer subjects and audited subjects. The reconstruction
frontier reduces record mutation/allocation work; index reconstruction and kernel
validation still visit the complete candidate. It does not claim effective audit
reuse or an interactive latency improvement.
`edit_frontier` separates source/syntax changes and declared fact changes from
derived consequences. The verification-only `full_rebuild` bypasses lowering
and prior producer caches while retaining identical parsed source identity inputs.
The oracle rebuilds all authored lowering, kernel records, producers and audits;
it does not allocate different syntax identities. The separate durable restore
gate reparses source bytes and checks/restores their exact syntax identities.

The `verification` feature records actual producer evaluations and inspects
physical kernel table ownership. It supplies no acceptance authority. See
[the integration test contract](tests/README.md) for accepted-cache tests,
including self-model edits and 100 mixed documents across five revisions with
parallel readers. No persistence, server adapter or Gen1 storage is included.
