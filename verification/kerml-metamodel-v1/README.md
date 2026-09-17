# KerML metamodel foundation verification

Branch: `foundation/kerml-metamodel-v1`, based on fetched main
`63c51a1` (semantic-kernel milestone merged). Run on Windows x64, Rust/Cargo 1.92.0
and Node 22.11.0 on 2026-09-17. Exact times, command lines, exit codes, source hashes
and log paths are in [results.json](results.json). All recorded commands passed.

| Check | Final result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass, 97 tests including the kernel doctest |
| `npm run check` | Pass |
| `npm run build` | Pass |
| `npm test` | Pass, 6 tests |
| `npm run test:e2e` | Pass, 4 browser tests |
| `cargo test --locked --offline -p agq-metamodel-gen` | Pass, 10 importer tests |
| `npm run metamodel:check` / equivalent offline Cargo command | Pass, exact committed IR matches |
| `npm run standards:check` | Pass, original standards/libraries plus all 3 new normative artifacts |
| `npm run format:check` | Pass |
| `cargo tree --locked --offline -p agq-metamodel-gen --edges normal` | Recorded; no transport/database/async/language frontend dependency |

Cargo ran with `CARGO_NET_OFFLINE=true`. Tests use local fixtures, pinned standards,
and a loopback browser server. Dependency provisioning and one-time artifact
acquisition are separate from build/test execution. The acquisition script was
not called by verification. This report does not claim a network-isolation sandbox.

The first full suite had 96 Rust tests and 9 focused importer tests. Final review
added a regression for documentary text split by an XML comment and corrected
text retention. `post_review` in results.json records those two changed source
hashes and the repeated format, workspace Clippy/tests, focused importer tests and
IR check. The final Rust total is 97 and the focused importer total is 10. The
generated IR and all runtime/frontend sources were unchanged by that correction.

Importer coverage includes deterministic repeated import and separate CLI output,
a frozen identity encoding/UUID, version/artifact/kind separation, duplicate simple
names across packages, multiple inheritance, multiplicity/defaults, composition
fixture metadata, redefinitions/subsetting/derived unions, association-owned ends,
opaque rules, missing/wrong-kind references, duplicate IDs, unknown structural
input, malformed XML/booleans/bounds, cycles, DTD rejection, a JSON mismatch fixture,
actual normative inputs, stale output, input corruption and output-over-input
refusal. Failed checks preserve existing bytes.

The committed IR contains 82 classes, 2 enums, 131 associations and 313 properties.
The JSON comparison checks all 210 class-owned properties and 88 direct class
inheritance edges. Its 36 scalar nullability differences are recorded explicitly;
comparison success is not execution or semantic verification. There are no
unhandled structural forms in the pinned inputs. Operation/constraint semantics
remain opaque. See [ADR 0002](../../docs/adr/0002-normative-metamodel-pipeline.md).

Generated IR SHA-256:
`d9e604c55d4aae966d3e9afddfc87f87e2b8f025d1ba0d8f43ef7f1b674d7a59`.

The main verification runner is wired to the metamodel check. This milestone ran
the checks listed above, not the broader `npm run verify` pilot-validator/demo
sequence. Existing independent-validator disagreements in other evidence are not
reclassified by this report. Historical browser screenshots/results in the root
verification directory were restored after archiving this run's browser report.

The exact changed/new file inventory is [files.txt](files.txt). `git diff --check`
passed; `crates/kernel`, supplied PDFs/HTML and existing standards/library bytes
have no diff. Next decisions concern runtime redefinition/subsetting/union and
opposite semantics, authoritative containment, primitive/enum/default domains,
JSON nullability and cross-release identity migration. No runtime descriptor set
was generated.
