# Declared ownership projection

Implementation: `b959ad1`, `80fca95`. Verification: `2c0971f`, including the
independent formal-owner absence guard. No publication claim.

Appending an unrelated owned relationship changes the aggregate property's
derived explanation. A filtered query previously imported that entire explanation
even when every selected relationship was already in the original declared slot.
The kernel now exposes that original stored slot without copying or replacing it.
Filtered queries may cite its declared fact only when it contains every selected
reference. Typed current-population searches, selected identities and canonical
positive evidence remain intact. Later-only and mixed selections keep the full
derived aggregate proof; empty selections invent no property fact.

Private producer queries separately track expanded current derived facts. A prior
declared subset observation therefore cannot suppress a subsequent broader read.
Regression controls cover both observation orders, public/producer modes, original
origins, mixed selections, absence, materialized-output rereads and archive restore.
The stored `Dependency::Declared` remains an original contribution on reread.

Windows low-artifact environment and isolated `target/foundation-closure` as in
`scheduler.md`. Actual checks, all exit 0:

| Command | Result |
| --- | --- |
| `cargo test -p agq-kerml-semantics --lib filtered_declared_ownership -- --nocapture` | 1 passed |
| `cargo test -p agq-kerml-semantics --lib producer_closure_tests -- --nocapture` | 41 passed |
| `cargo test -p agq-kernel --test archive -- --nocapture` | 6 passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy -p agq-kernel -p agq-kerml-semantics --all-targets --locked --offline -- -D warnings` | Passed |
| `RUSTDOCFLAGS=-D warnings cargo doc -p agq-kernel -p agq-kerml-semantics --no-deps --locked --offline` | Passed |

The 60,000-subject / 80-family scale fixture retained 2,220,208 certificate bytes;
debug construction took 6,445 ms. This does not measure full publication cost.
