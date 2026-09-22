# Dependent graph archive preparation

This neutral kernel transport carries no language publication acceptance. A dependent
overlay archive contains local declarations and derived differences, with a digest of
the separately supplied root dependency archive. Restoration retains that exact
dependency `Arc`, including canonical record allocations and protected ownership.
The existing root archive format remains unchanged.

The regression verifies a larger combined registry, exact graph round trip, stable
archive bytes, local-only record serialization, retained retired-ID reservations,
wrong registry/dependency rejection, and rejection of dependency rewrites or ownership
acquisition through a new local record.

Verification on 2026-09-22, Windows, Rust/Cargo 1.92.0. Environment:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2`.
Target: `target/foundation-actions` in the root integration checkout.

| Command | Actual result | Exit |
| --- | --- | --- |
| `cargo test -p agq-kernel --test archive -- --nocapture` | 6 passed, 0 failed | 0 |
| `cargo test -p agq-kernel` | All package tests and 3 doctests passed | 0 |
| `cargo clippy -p agq-kernel --all-targets -- -D warnings` | No warnings | 0 |
| `cargo fmt --all -- --check` | No diff | 0 |

No trusted Systems receipt, accepted binding manifest, or publication status was minted.
