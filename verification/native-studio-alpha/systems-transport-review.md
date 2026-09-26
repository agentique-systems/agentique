# Independent Systems transport-receipt review

Date: 2026-09-25. Reviewer: experience subagent, independent of the runtime
implementation stream. Scope: transport trust boundary and unchanged restoration
checks. No language semantics, accepted receipts or bindings were edited by this
review.

## Judgment

The reviewed design permits a versioned cache transport without replacing semantic
publication authority. No arbitrary caller-supplied receipt or unchecked graph can
become trusted through this path. The user explicitly authorized a versioned
transport receipt where exact archive-byte reproduction is unavailable, provided
the accepted semantic contract is independently preserved.

This is conditional approval of the transport mechanism, not acceptance of a
particular regenerated Systems cache. At review time the production catalogue's
alternate list was empty. Actual activation requires a concrete reviewed transport
pin and successful ordinary restoration against the original accepted identity.
Real Agentique first light remains a separate product gate.

## Reviewed implementation

The implementation under review is `crates/kerml-semantics/src/trusted_publication.rs`
in the runtime worktree. Its private `TransportReceipt` is deserializable only as
part of a finite compiled catalogue; it is not a public authority constructor.
The `TransportReceipt` and `TransportEntry` structures reject unknown fields.
The reviewed, not-yet-committed file had SHA-256
`153caca8d54ad6afe0468d9c060ec043d04453413c993a5d24f0bc44870d6aca`.
Later changes and a concrete transport registration require followup review.

Catalogue loading requires:

- The exact versioned format, matching publication catalogue ID, nonempty unique
  transport ID and SHA-256 anchor of the original accepted receipt's raw bytes.
- Exact equality of all accepted identity fields.
- Exact entry names and original uncompressed entry lengths.
- Exact original hashes for all entries except the one explicitly permitted by
  the private catalogue definition: `kernel.jsonl`.
- An explicitly compiled alternate digest for that kernel entry. A third digest
  remains rejected. The original digest remains valid.

The original receipt and binding object remain stored unchanged. Semantic identity,
source identity, profile, entry lengths, closure evidence and bindings are still
read from those objects. A transport document cannot add a new semantic identity,
replace bindings, change the closure bytes, widen the profile or redefine source
freshness. Since only one entry can differ, there is no cross-entry combination of
different transports that could create an unreviewed cache combination.

The independently read original Systems authority SHA-256 is:

```text
becc3cf991e69115dae905957d74292774164e3f5707eecefb06d8eb60e0269a
```

The repository's LF text policy makes the raw-byte anchor stable across supported
checkouts. An actual receipt-byte change would invalidate the alternate anchor;
it is not silently normalized by the new loader.

## Independent graph, evidence and source checks retained

`CanonicalSysmlSystemsLibrary::restore_with_authority` authenticates the selected
kernel entry before decoding, then re-encodes the decoded graph and authenticates
that actual graph again. This protects the existing boundary against an input
changing between the initial hash and decode. An allowed transport hash is only
the first gate.

The existing producer context computes the semantic graph identity from the actual
restored model. Its encoding includes element and association identities, classes,
values, provenance, proof dependencies, derived navigation, failures and structural
searches; the producer form also binds original declared slots. Closure restoration
compares that model digest, the independently reconstructed registry digest and
context contract with the original receipt. It verifies the exact original
closure bytes and reconstructs the certificate. The certificate's ordered subject
IDs must equal the actual model's element IDs, its own digest must match and every
semantic closure requirement must be covered. Finally the SysML publication digest
and full binding manifest are checked again.

Snapshot revision labels are deliberately excluded from the existing closure
context digest. The publication identity is derived from semantic identity and
the accepted dependency/profile/binding contract rather than that random label.
The versioned transport therefore does not need to invent an unavailable original
snapshot UUID.

The existing source boundary remains intact: `VerifiedLibrarySet` verifies source
bytes; interpretation checks compare source-content identity, Systems artifact
identity, accepted KerML dependency, operational profile, rule set, descriptor
graph and compatibility/correction manifests. Facade validation checks the exact
document population, hashes, profile and source provenance. No new transport field
can override those checks. The guarded historical producer runner supplies an
additional recorded source-provenance chain, rather than replacing restoration.

## Evidence limits and activation conditions

The original Systems receipt pins the kernel payload hash but does not retain its
random snapshot revision. Without the original payload, a different regenerated
kernel hash cannot be proved to result solely from that UUID. A fresh fixed-width
UUID is an explanation consistent with the frozen producer and equal size; it is
not a byte-diff observation. Documentation must distinguish that inference from
the semantic equality independently established by restoration.

The existing semantic digest is also not a digest of every byte of archive
bookkeeping: for example, retired identity reservations and serialized reference
contribution tables are distinct storage structures. The new mechanism must not
be described as accepting arbitrary graphs with a claimed matching semantic
digest. It accepts only a finite reviewed transport hash from the exact historical
producer, then independently checks the accepted semantic graph and evidence.
Ordinary canonical archive validation remains mandatory. No semantic encoder was
changed to accommodate rematerialization.

Before activating a concrete alternate:

1. Retain exact historical producer/source checks and the generated artifact hash.
2. Require complete accepted identity and bindings equality, unchanged facade and
   closure entries, and unchanged kernel entry length. Any mismatch outside the
   permitted kernel hash stops this path.
3. Add only that concrete digest in a separately retained versioned transport
   document anchored to the original receipt, preserving the original documents.
4. Run the current ordinary facade restoration, package verification and subsequent
   real-model acceptance. Record failures as failures, not transport exceptions.

## Review and test evidence

The reviewer inspected the implementation diff and the existing SysML restoration,
semantic graph encoder, producer context, closure restoration, source validation
and guarded rematerialization paths. `Get-FileHash` independently confirmed the
original receipt hash above, exit code 0. No duplicate Cargo build was launched by
the reviewer; the runtime owner is running the focused Rust gate. Passing transport
unit tests alone cannot establish a real cache's semantic restoration.

At this document's commit, that focused gate was still pending. No passing Rust
test result is claimed here. `git diff --check` for this review document completed
with exit code 0. Followup review should bind the final implementation commit,
actual focused test result and concrete transport pin before runtime first light.

## Followup: final inactive mechanism

Independently reviewed commit `5007ea5e9b23945cc8d72cc31c936dddd777822a`.
The production alternate list remains empty. The implementation keeps the same
boundary described above, with explicit negative cases for facade and profile
replacement and an unregistered kernel digest. No new authority bypass was found.
Its focused Rust gate was still compiling at this followup; no Rust pass is claimed
here. Review of a concrete transport registration and actual facade restoration
remains outstanding.

The reviewer independently reran `python verification/scripts/test_runtime_rematerialization.py`
in the runtime worktree: 13 tests passed in 0.089 seconds, exit code 0. The runtime
owner subsequently reported the focused Rust tests passing in debug and release,
with evidence committed as `b9a432e`; those Rust commands were not rerun by this
reviewer. No actual alternate transport has yet been registered or accepted.
