# Structural proof transport

Implementation: `299afc6`. Verification: `dae44d8`, which also includes the
independently tested ownership requirement mapping. No publication claim.

Namespace/import/redefinition reads previously became broad kernel `Element`
searches when stored with a derived fact. Their native causal interpretation was
structural, so persistence incorrectly introduced dependencies on unrelated
primitive scalar producers. The appended language-neutral
`RelationshipStructure` search preserves that native interpretation; semantic
digest tag 12 and the archive enum are append-only. Existing `Element` searches
remain broad, including in restored older evidence.

A real namespace/import/alias query has identical native and persisted causal
reads. Declared names, optional alias names, membership/import visibility,
recursive imports and import-all flags remain explicit scalar dependencies.
Structural membership/naming/ownership changes still invalidate the population.
Tests also cover both imported-namespace scopes, redefinition scopes, ordinary
revision invalidation, retained provider obligations, exact subject digest
identity and archive roundtrips.

Windows low-artifact environment and isolated `target/foundation-closure` as in
`scheduler.md`. Actual checks, all exit 0:

| Command | Result |
| --- | --- |
| `cargo test -p agq-kerml-semantics --lib transport -- --nocapture` | 6 passed |
| `cargo test -p agq-kerml-semantics --lib derived_proofs_searches_navigation_and_failure_details_are_included -- --nocapture` | 1 passed |
| `cargo test -p agq-kernel --test archive -- --nocapture` | 6 passed |
| `cargo test -p agq-kerml-semantics --lib producer_closure_tests -- --nocapture` | 39 passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy -p agq-kernel -p agq-kerml-semantics --all-targets --locked --offline -- -D warnings` | Passed |
| `RUSTDOCFLAGS=-D warnings cargo doc -p agq-kernel -p agq-kerml-semantics --no-deps --locked --offline` | Passed |

The repeated 60,000-subject / 80-family scale fixture retained 2,220,208 certificate
bytes, 300,000 applicable pairs and 299,995 closed pairs; debug construction took
6,284 ms. One incomplete writer blocks four dependent evaluations.
