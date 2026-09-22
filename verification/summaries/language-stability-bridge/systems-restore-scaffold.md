# Exact Systems restoration scaffold

Status: **disabled pending actual accepted Systems receipt and bindings**.
No placeholder authority, accepted facade or standard bindings were minted.
The new catalogue is empty; `checked_in("sysml-systems-operational-v2")` fails.
The current accepted KerML `/26` receipt, restoration implementation and default
SysML profile are unchanged.

`TrustedPublicationReceipt` is opaque and selects only compiled documents. It
authenticates bounded closure bytes against their independent receipt before
decoding, verifies the actual model/context and complete registry, then calls the
existing private certificate decoder. It has no caller-data authority constructor.
The SysML context bridge independently rebuilds the combined registry.

`CanonicalSysmlSystemsLibrary::restore_cache(reader, sources, accepted_kerml)`
checks the exact three archive entries and trusted uncompressed sizes/hashes;
metadata and closure are decoded from those authenticated bytes. The dependent
graph is bounded during decoding, then re-encoded and hashed to reject input
substitution after the first archive read. Restoration retains the supplied
KerML dependency `Arc`, checks original source/document/provenance populations,
rebuilds public canonical bindings, and recomputes context/publication identity.
It reconstructs historical audit counts separately from zero local producer
counters and performs no producer replay.

The implementation was rebased onto the authenticated immutable-dependency
bridge `de501b5` and uses its composed KerML context accessor. The public binding
visibility field already added by the lead is preserved.

Focused checks cover three closure-authority tests and three archive/metadata
tests: exact private-fixture certificate restoration, forged bytes and recomputed
weaker certificates, changed context, weaker registry, changed bindings, truncated
or oversized input, missing/extra/duplicate/aliased entries, stale documents and
omitted reference/capability populations. Private fixtures cannot create an
accepted Systems facade through the public API.

Formatting, all-target Clippy for the three changed packages, and Rustdoc with
warnings denied pass. All five existing KerML restoration regressions also pass,
including its accepted graph roundtrip and dependency protection.
`systems-restore-preparation.json` records exact commands,
source/patch identities, actual exits and output hashes, including the initial
compile/lint corrections. Low-disk builds used two jobs, no debug symbols and no
incremental artifacts; no caches were deleted.

Remaining activation work is deliberately tied to the real publication gate:
check in its independently accepted receipt and binding documents, add that exact
catalogue entry, and run the complete accepted-cache roundtrip and adversarial
matrix against the actual graph. Confirm identical standard/publication IDs,
shared KerML allocation, zero producer replay, and integration with the authored
acceptance fixtures before reporting C2 complete. Preparation tests cannot stand
in for those checks.
