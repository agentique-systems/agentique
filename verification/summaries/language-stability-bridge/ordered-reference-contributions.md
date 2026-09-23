# Precise ordered-reference append evidence

Kernel commits `b6bcfaa` and `4fa87cd` retain optional immutable evidence per
`(element, property, target)` appended reference. Each entry keeps the caller's
exact append explanation plus owner/target existence, its position, and only the
current append batch's searches. Earlier aggregate proofs/searches and automatic
existence dependencies of other batch targets are excluded. Ordinary aggregate
slot provenance remains complete and unchanged.

`ModelView::ordered_reference_contribution` and its deterministic iterator expose
borrowed kernel-created entries. A later aggregate search submission without an
actual append evicts precise entries because those new reads cannot be attributed
to an earlier event. Additive strict/construction builds, construction promotion,
and exact immutable dependency mounts preserve entries without mutating old
readers. Initial derived ordered slots also capture per-target evidence rooted
in the validated owner creation proof, including record and slot negative search
inputs. Repeated initial targets in nonunique ordered slots stay aggregate because
one target key cannot identify multiple positions. Original declared entries use
their original declared evidence; absent optional metadata requires aggregate
evidence.

Archives retain their existing format and canonical bytes; local contribution
metadata is omitted. Restored local entries return `None`, requiring aggregate
query fallback and invalidation of transported evidence that used the missing
entry. The new `StructuralSearch::OrderedReferenceContribution` identifies that
separate availability/proof/position dependency; semantic readers must also retain
their current population searches and recursively follow positive proof inputs.
Evidence on an independently supplied exact immutable dependency remains shared.
This change does not alter historical accepted KerML interpretation or acceptance.

Actual verification (2026-09-23):

| Command | Result | Exit |
| --- | --- | --- |
| `cargo test -p agq-kernel --test reference_contributions` | 7 passed | 0 |
| `cargo test -p agq-kernel` | 119 package tests and 3 doc tests passed | 0 |
| `cargo clippy -p agq-kernel --all-targets -- -D warnings` | clean | 0 |
| `cargo doc -p agq-kernel --no-deps` with `RUSTDOCFLAGS=-D warnings` | clean | 0 |
| `cargo fmt --all -- --check` | clean | 0 |

The focused tests cover initial derived slot creation and its negative searches,
repeated-target fallback, distinct append batches, multiple targets in one batch,
declared-prefix/derived-suffix mixing, adoption with explicit contextual evidence,
copy-on-write and moved maps, search-only cache invalidation, construction and
immutable dependency mounts, local archive fallback, exact archive byte roundtrip,
and retained supplied-dependency evidence. No publication was run.

Build environment: `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, isolated
`CARGO_TARGET_DIR=../agentique/target/bridge-self-model`. The lead owns semantic
query integration and the exhaustive matches for the new structural search.
