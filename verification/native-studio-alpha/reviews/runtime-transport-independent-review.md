# Independent Systems transport review

Reviewer: editor/state reviewer, independent of runtime implementation. Scope:
`88a4b14`, `4131117`, `37f1e6d`, as present at main `e0488d7` on 2026-09-25.
This is a mechanism review, **not acceptance of a concrete runtime transport**.
The compiled alternate-transport list was empty. The historical Systems producer
was still running; no new Systems cache or proposed pin was available.

## Findings

1. **P2 — preparer can emit inconsistent outer-container evidence during a file
   replacement.** `prepare_systems_transport.py:30-36` hashes one open of the
   pathname, then reopens it for ZIP inspection; line 79 separately stats the
   pathname. A concurrent writer can make `content.json.cache_sha256` refer to
   different bytes than its entry digests and byte length. The independent
   temporary-fixture probe changed only the outer ZIP comment between these
   reads: preparation succeeded, every semantic payload remained identical, but
   the recorded and actual final outer hashes differed. This does **not** bypass
   the ordinary facade's entry/semantic authentication, and the emitted record
   still says `runtime_accepted: false`. It does undermine the promised exact
   review/distribution evidence. Requested correction: hash and inspect one
   handle, retain its counted length, and reject mutation by rehashing that same
   handle after inspection. Runtime team acknowledged and owns this fix. The
   concrete registration review must independently hash the retained final path.

No P0/P1 semantic-authority bypass found in this bounded review. The P2 was open
at `e0488d7`; runtime correction `fcd2fb2` has since been independently inspected
and the original mutation probe now rejects the candidate. All **20** focused
Python gates passed on that correction. See the
[follow-up evidence](runtime-transport-independent-review-fixed.json). The
single handle binds entry inspection, counted length and both outer hashes;
later facade authentication still checks the actual input. A subsequent pathname
replacement must still be caught by independent concrete artifact hashing.

## Trust boundaries checked

- `trusted_publication.rs:21-28,89-160`: the catalogue and alternate pins are
  compiled source. A cache/caller cannot add authority. Transport schema rejects
  unknown fields; the original semantic receipt's raw SHA-256, complete identity,
  exact entry set, original lengths and unique transport ID are required. Only
  `kernel.jsonl` may have a different digest. Facade and closure remain pinned to
  original bytes. The original receipt/bindings are not replaced by the transport.
- `trusted_publication.rs:226-263`: original closure bytes, independently derived
  registry and context contract, actual model semantic digest and certificate
  digest must agree. A new cache label cannot establish query completeness.
- `kerml-text/src/sysml/publication_restore.rs:244-280`: ordinary restore checks
  the source/dependency/profile contract, authenticates bytes before decoding,
  and re-encodes the decoded graph to demand an allowed exact digest and length.
  This protects the restored graph even if a seekable reader changes between
  authentication and decoding.
- `publication_restore.rs:281-347,383`: bindings are reconstructed from the
  actual overlay and verified source provenance; dependency identity and the
  original closure are reattached to that exact model. Fully closed coverage,
  semantic/publication identities and complete binding manifest are required.
  `sysml-semantics/src/context.rs:223-255` reconstructs the complete registry.
- `compare_systems_contract.py:17-43` compares every non-entry receipt field and
  the entire binding manifest with JSON type distinctions intact. Entry shape is
  constrained. `prepare_systems_transport.py:19-29,37-61` additionally requires
  original lengths, exact non-kernel entries, actual uncompressed hashes and a
  canonical snapshot UUID. Its output is explicitly a review candidate and its
  CLI has no catalogue/authority-writing operation.
- `runtime-rematerialization.md` correctly separates semantic authority, encoding
  and distribution. It does not claim the unavailable original Systems graph
  differs solely by its snapshot UUID. Reserved identities and reference
  contribution encoding extend beyond the canonical semantic digest. This is
  why the frozen producer provenance and an explicitly reviewed exact transport
  pin remain necessary, in addition to semantic equality.
- The runtime runner checks the exact historical source commit, clean tracked
  diff and retained producer executable SHA before execution; the finalizer's
  journal is separately hashed. These are external provenance controls, not
  guarantees supplied by the preparer. The preparer alone is insufficient for
  registration. `.gitattributes` fixes LF for the accepted receipt, avoiding an
  accidental platform-specific raw receipt hash from Git line-ending conversion.

## Evidence and pending concrete gate

[Machine-readable review evidence](runtime-transport-independent-review.json)
retains reviewed source SHA-256 values, exact command/output and exit code 0:
**19 existing Python gates passed**. The
[independent reproducer](runtime-transport-independent-review.py) modifies only a
temporary fixture. No producer, authority, catalogue or parent source was edited.
No additional Rust build was run during this bounded review.

Original receipt raw SHA-256 observed independently:
`becc3cf991e69115dae905957d74292774164e3f5707eecefb06d8eb60e0269a`.
Original binding file raw SHA-256:
`dbd794bba9ebdfdea5af433945f6bb5d0475f0f05ad1d229cd004f5b57045789`.

When available, the separate concrete gate must inspect the actual cache, producer
and finalizer evidence, complete contract comparison and all 69 accepted bindings;
independently rehash all entries; require original facade/closure hashes and
`kernel.jsonl` length **748560767**; compare the proposed compiled transport's
entire identity and original authority hash; confirm only the new exact kernel
digest is introduced; and inspect ordinary facade authentication of that actual
cache after registration. Packaging/install/restart must authenticate the same
retained bytes. A real-model acceptance claim additionally requires the native
Agentique repository journey; passing this mechanism review cannot substitute.
