# Checked construction-to-strict revalidation

Implementation `9921542`, semantic digest fixture `40bb235`; fetched milestone
base `67fdddc81e5aceacea7cb407cdb2f45d621fd674`.

`ConstructionOverlay::revalidate(self, declared: Snapshot)` checks exact original
declarations, record/occurrence provenance, descriptor contracts, retired/reserved
identities, navigation/search/failure metadata and immutable dependency Arc. A
fresh strict revision label is allowed only when all these inputs agree.

The ordinary strict derivation builder rechecks the entire structural graph,
including lower bounds and protected ownership. Revalidation then checks every
proof dependency, unsuccessful dependency boundary, search subject and factored
proof cycle. It evaluates no language producers and issues no semantic acceptance.
Missing required derived values fail strict validation. Existing incomplete
semantic computations remain incomplete; storage validity does not erase them.

Canonical record, proof and search allocations remain shared; consuming the last
construction-overlay handle transfers owned maps. The independently committed
declared snapshot contains no generated records. The semantic graph digest is
unchanged across a successful revalidation, including when revision labels differ.

Verification used Cargo/Rust 1.92.0 and the low-artifact variables documented in
`query-evidence.md`, with isolated `target/foundation-evidence`.

| Actual command | Exit | Observed output |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kernel --test construction_derivation --test derived_associations` | 0 | 11 + 12 passed |
| `cargo test --locked --offline -p agq-kerml-semantics --test producer_closure_evidence strict_construction_revalidation` | 0 | 1 passed, 2 filtered |
| `cargo test --locked --offline -p agq-kernel` | 0 | All package tests passed, including 60k derivation scale and 3 Rustdoc tests |
| `cargo clippy --locked --offline -p agq-kernel --all-targets -- -D warnings` | 0 | No warnings |
| `cargo doc --locked --offline -p agq-kernel --no-deps` with `RUSTDOCFLAGS=-D warnings` | 0 | Documentation generated |
| `git diff --check` | 0 | No whitespace errors |

An initial test compilation used the borrowed search set where `Arc::ptr_eq`
requires a shared handle; the corrected test retrieves both canonical handles.
The successful runs above use that correction.
