# KerML typed views verification

Branch: `foundation/kerml-typed-views`.
Base commit: `43964675bcda087e2afb75cda9848cd792d4bfe2`.
Executed on Windows on 2026-09-17; command timestamps, durations and exit statuses
are recorded in [results.json](results.json). All final commands exited 0.

| Command | Actual result | Log |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | [format.txt](format.txt) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed, no warnings | [clippy.txt](clippy.txt) |
| `cargo test --workspace` | 123 passed, including two rustdoc tests; zero failed/ignored | [rust-tests.txt](rust-tests.txt) |
| `npm run check` | Passed | [frontend-check.txt](frontend-check.txt) |
| `npm run build` | Passed | [frontend-build.txt](frontend-build.txt) |
| `npm test` | 6 passed | [node-tests.txt](node-tests.txt) |
| `npm run test:e2e` | 4 passed | [browser-tests.txt](browser-tests.txt), [browser-results.json](browser-results.json) |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | All four artifacts current | [generated.txt](generated.txt) |
| `cargo doc --no-deps -p agq-kerml -p agq-kernel` with `RUSTDOCFLAGS=-D warnings` | Passed | [rustdoc.txt](rustdoc.txt) |
| `cargo tree -p agq-kerml -e normal` | Only direct dependency is `agq-kernel` | [dependencies.txt](dependencies.txt) |

The new tests cover borrowed record/string/collection identity, ten programmatic
normative records including first-class relationships, checked upcasts and invalid
downcasts, registry extensions, property identity independent of display names,
scalar redefinitions through a collection accessor, ordered/unordered and
nonunique values, malformed values, absent versus present-empty versus uncomputed
properties, immutable old snapshots and all generated IDs. Generic association
tests prove a single canonical endpoint slot and reject inverse bounds, ordering,
navigation, two class ends and nonunique authored ends. Existing ownership-write
refusal tests still pass.

An initial focused run found that rustfmt changed the generated metamodel constant
layout. The emitter's formatting boundary was corrected; deterministic generation,
read-only stale-file detection and the complete final suite then passed. No failing
checks remain in the final run above.

Reproduce the recorded command set from the repository root with
`node verification/kerml-typed-views/run.mjs`. It records exit statuses even if a
command fails and exits nonzero if any command fails. Browser result JSON is kept
here; preexisting shared browser reports/screenshots are restored after the run.
Logs concatenate each command's stdout and stderr without implying stream order.

See [ADR 0004](../../docs/adr/0004-kerml-typed-views.md) for API usage and scope.
These are structural and regression checks, not full KerML semantic verification.
Two writable ownership ends, rule evaluation, transitive semantic inheritance,
namespace resolution, parser, persistence and simulation remain outside this slice.
