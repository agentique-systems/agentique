# Authenticate the exact runtime decoder stream once

The preceding real warm-open profile spends 3.842 seconds reading/hashing the
KerML graph before a separate 16.138-second graph decode. SysML has the same
duplicate decompression structure. Earlier read-profile-02 measured 5.626 seconds
across the two input authentication passes. This bounds the opportunity; hashing
still has to run, and no measured speedup is claimed before a new runtime profile.

`AuthenticatedInput` now counts and hashes the actual bytes consumed by the
ordinary kernel decoder. Each archive entry's declared length must match the
independently trusted publication receipt. The input allows at most that length
plus one byte and requires a real EOF probe, exact byte count, and no unhandled
I/O/CRC failure. Empty reads cannot establish EOF. The decoder already rejects
content after its End record, including buffered whitespace. Authentication never
drains an unread suffix to make a partial parse succeed.

Both facades verify this raw digest before accepting any publication. Their
independent re-encoding of the decoded graph, proof/contribution and dependency
authentication, semantic contexts, source identities, binding manifests and
producer closure checks are unchanged. A same-length UUID case mutation can
decode to identical canonical bytes but produces a different input digest; the
new regression demonstrates why both checks remain necessary.

This moves allocation/structural decoding before the final raw-digest decision
for malformed same-size input. The trusted total size and kernel line bounds
remain in force. No accepted handle escapes before all checks finish.

Independent read-only review by `/root/interaction` found no acceptance bypass.
Unit coverage includes interrupted/irregular reads, no EOF, zero-length reads,
short/excess input, bound overflow, swallowed terminal errors, an actual corrupted
ZIP CRC, trailing kernel content and normalization-distinct raw bytes. The text
suite passed 72 tests with four accepted-runtime gates ignored; the added
normalization case subsequently passed its exact test. Initial compile/lint
failures (missing mutable test variable and a Clippy style issue) and the later
passing outputs are retained under `../../native-studio-alpha/checks/`.

Runtime measurement, accepted-runtime audit interruption and enabled-control
command/cold exact equivalence remain separate gates. No accepted publication
receipt or semantic identity pin changed.
