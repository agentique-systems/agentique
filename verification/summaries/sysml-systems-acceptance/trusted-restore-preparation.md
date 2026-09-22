# Trusted Systems restoration preparation

Status: **implemented scaffolding, disabled; pending actual Systems publication
acceptance**. The bridge milestone explicitly authorized preparing this code in
isolation. Its finite compiled catalogue is empty, so no Systems cache can restore
an accepted facade yet. Add catalogue authority only after the complete publication
gate passes. Existing Systems export bytes alone remain insufficient authority.
The accepted KerML Operational v9 `/26` receipt encoding and restoration API are
unchanged. See
[`systems-restore-scaffold.md`](../language-stability-bridge/systems-restore-scaffold.md)
for implementation checks and the remaining accepted-artifact gate.

## Authority and concrete API

Add `crates/kerml-semantics/src/trusted_publication.rs`, exported by `lib.rs`.
An opaque capability selects an entry from a finite compiled catalog containing
only independently checked-in accepted receipt and binding documents. Catalog
entries contain data, with no dependency on SysML implementation crates. Add the
Systems entry only from the actual accepted export; do not add a placeholder.

```rust,ignore
pub struct TrustedPublicationReceipt { /* private authenticated documents */ }

impl TrustedPublicationReceipt {
    pub fn checked_in(id: &str) -> Result<Self, TrustedPublicationError>;

    pub fn restore_producer_closure(
        &self,
        reader: impl std::io::Read,
        context: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> Result<Arc<ProducerClosureCertificate>, TrustedPublicationError>;
}
```

The identifier selects an existing catalog entry; it cannot supply authority.
No public JSON/hash constructor, `Deserialize`, mutable document access, or
caller-implemented authority trait may mint this capability. The existing
certificate decoder remains crate-private. The restoration method bounds and
hashes actual closure bytes before parsing, then verifies the pinned model,
registry, context-contract and certificate digests before calling that decoder.
It accepts a model-bound `SemanticContext`, never a caller-built
`SemanticContextId` or claimed graph digest.

Add the narrow bridge in `crates/sysml-semantics/src/context.rs`:

```rust,ignore
impl<'m> SysmlSemanticContext<'m> {
    pub fn with_trusted_producer_closure(
        self,
        receipt: &TrustedPublicationReceipt,
        reader: impl std::io::Read,
    ) -> Result<Self, SysmlContextError>;
}
```

The bridge verifies the Systems authority selection and independently assembles
the expected KerML plus SysML producer registry. It binds that registry, calls
restoration with its private inner context, then reuses certificate attachment
validation. This grants exact-context query evidence, not library acceptance.

Add the facade operation in
`crates/kerml-text/src/sysml/publication_cache.rs`:

```rust,ignore
impl CanonicalSysmlSystemsLibrary {
    pub fn restore_cache(
        reader: impl std::io::Read + std::io::Seek,
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
    ) -> Result<Self, SystemsPublicationCacheError>;
}
```

Only private facade construction may establish accepted Systems status after all
checks. No public facade constructor takes restored status labels or unchecked
parts. No new generic protocol or crate is needed.

## Restore sequence

1. Select compiled authority and authenticate its complete binding manifest.
2. Check exact verified source set, Systems KPAR, accepted KerML identity,
   Operational v2, grammar/correction manifests and combined descriptors.
3. Require exactly `facade.json`, `closure.json`, and `kernel.jsonl`, rejecting
   duplicate/extra names. Check trusted uncompressed lengths and byte hashes
   before allocating decoded data.
4. Use `read_dependent_overlay` with the independently constructed combined
   descriptor registry and the supplied accepted KerML overlay `Arc`.
5. Re-encode the actual restored dependent overlay and verify its pinned archive
   hash. A caller-supplied changing seekable reader or a substituted decoded
   graph must not bypass validation. Parse metadata/closure from the same
   authenticated bytes, or independently recheck their parsed content.
6. Validate metadata, canonical source-map order, duplicate rejection,
   provenance, all 21 document identities, exactly 1,327 mandatory references,
   and the complete checked-family population against the trusted receipt.
7. Rebuild all standard bindings through `StandardSysmlBindings::validate` and
   verified-source checks. Reconstruct the final composed dependency/context
   contract from those bindings and current pinned authority.
8. Restore and attach the authenticated closure certificate under the complete
   independent producer registry. Recompute semantic/publication identities and
   the binding manifest, requiring exact receipt agreement.
9. Construct the accepted facade privately. Preserve the supplied dependency
   `Arc` and canonical IDs. Record restoration separately from historical run
   telemetry; local producer stages/counters show zero replay.

Do not rerun producers, reconstruct derived facts as declared facts, flatten the
shared dependency, or treat authenticated audit labels as authority independent
of the receipt and exact graph checks.

## Adversarial restore matrix

| Fixture | Required result |
| --- | --- |
| Exact accepted cache, sources and dependency | Restores with identical identities, shared KerML `Arc`, zero producer replay |
| Cache claims `accepted`, or supplies a new receipt with recomputed hashes | Reject; only compiled catalog documents authorize restoration |
| Altered closure masks/states, subjects, or self-consistent certificate digest | Reject against trusted closure-entry hash and pinned certificate identity |
| Matching graph with weaker KerML-only producer registry | Reject against independently assembled combined registry |
| Stale graph, context contract, registry, profile, grammar or correction manifest | Reject the specific identity mismatch |
| Changed original source bytes, Systems KPAR, document population or source map | Reject verified-source/metadata mismatch |
| Missing, repeated or stale role binding; changed canonical ID/source/metaclass | Reject binding manifest or independently rebuilt anchors |
| Different accepted KerML graph/dependency | Reject before issuing accepted facade or closure evidence |
| Missing, duplicate, extra, truncated or oversized archive entry | Reject bounded archive ingestion |
| Reader substitutes graph after initial hash; decoded graph differs | Reject actual restored overlay re-encoding mismatch |
| Missing reference assertion or incomplete checked-family population | Reject exact accepted audit population |
| Existing accepted KerML `/26` cache | Restores unchanged through its existing authority/API |

This document originated as read-only preparation. The linked bridge summary
records the subsequent implementation tests. Neither stage establishes Systems
publication acceptance or language readiness.
