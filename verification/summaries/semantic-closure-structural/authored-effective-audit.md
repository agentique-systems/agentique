# Authored effective audit integration

Prepared on `0fcc0ff` after the accepted Systems publication and language
stability gate. No semantic producer, canonical rule, operational profile or
finalizer acceptance criterion changes here.

`SourceInputs::compile` now invokes the existing strict effective SysML audit
dispatcher for every local canonical ID, including derived IDs absent from the
source map. Accepted dependency IDs are excluded. Deterministic batches of 32
reuse the exact immutable compilation context and release evaluator caches
between batches. No standard records or inherited elements are copied.

The private `SourceEffectiveAudit` fields retain the actual context, audited
local IDs and report. Public access is borrowed and read-only. The existing
native capability diagnostics retain their evidence; additional audit findings
retain native diagnostics and current source origins when available. Derived
subjects keep canonical identities without synthetic source locations.

`WorkingProjectRevision::validate` additionally requires a present, context-matched
report with no findings. Its original syntax, strict construction, references,
producer closure and compatible certificate gates remain. The report is created
for the final frontier and never copied from the previous revision.

The new accepted-cache test
`closed_malformed_attribute_typing_stays_working_until_repaired` covers an
AttributeUsage explicitly typed by a PartDefinition. It requires parsed inputs,
resolved references and Complete producer closure while the actual effective
attribute definition query remains Invalid and validation rejects. A source edit
repairs the definition kind; the new revision must validate while the earlier
failed revision, its audit, source evidence and borrowed query stay unchanged.
The fixture also verifies full local audit coverage, including derived records,
and the existing physical shared-standard storage assertions.

Local verification (PowerShell; isolated `target/authored-effective-audit`,
`CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, development/test debug info disabled):

| Actual command | Exit | Result |
| --- | ---: | --- |
| `cargo fmt --all -- --check` (initial) | 1 | Formatting differences in the two new edits; corrected with rustfmt. |
| `rustfmt --edition 2024 crates/kerml-text/src/source_inputs.rs crates/modeling-workspace/tests/working_states.rs` | 0 | Formatting applied. |
| `cargo check --locked --offline -p agq-modeling-workspace --all-targets --features verification` | 0 | All workspace targets, including the new accepted-cache fixture, compiled; 36.61 seconds. |
| `cargo fmt --all -- --check` | 0 | No output. |
| `cargo clippy --locked --offline -p agq-modeling-workspace --all-targets --features verification -- -D warnings` | 0 | No lint findings; 17.00 seconds. |
| `cargo clippy --locked --offline -p agq-kerml-text -p agq-modeling-workspace --lib --features verification -- -D warnings` | 0 | Both changed libraries clean; 0.15 seconds. |
| `git diff --check` | 0 | No output. |

Raw outputs remain under ignored `verification/generated/authored-effective-audit`.
PowerShell renders Cargo stderr progress as NativeCommandError text in redirected
logs; the captured process exit codes above are zero. No accepted-cache runtime
test was run in this worktree. The lead coordinates those memory-intensive gates
after integration; compilation is not recorded as runtime acceptance.
