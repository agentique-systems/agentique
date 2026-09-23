# Positive specialization and positioned population review

**GO for the next bounded Actions audit.** Independent review and focused checks
passed on closure correction `e3e92e0` (review-worktree equivalent `645f69e`, root
equivalent `b509a81`), with selected-origin cache correction `522b17c`. This does
not establish Systems publication or language readiness acceptance.

The review covered the positive-path helper, both SysML redundancy sites,
positioned parameter/end/result populations, and native/persistent existing-ID
reads through certificate reconstruction. No additional conformance or corpus run
was performed.

The selected canonical path retains exact endpoints and derived-edge provenance.
Conjugation, pending providers and implied-edge filtering remain guarded; virtual
and chaining-only reachability returns no witness rather than an absence claim.
Changed or removed selected edges reopen retained evaluations. The former
own-output shortcut is absent; materialized outputs use the same checked path.

The draft proposal suppression omitted the surviving proposal's transitive
antecedents when planning B,C,A with target identities A<B<C and ancestry A→B→C.
Ascending candidate order now chooses the retained smallest reachable proposal;
the selected path and its off-subject antecedents are merged. Both the owner-edit
and B,C,A chain regressions pass.

Positioned populations preserve selected carrier proofs and typed population
searches. Excluded candidates retain scalar and identity guards without importing
irrelevant creation proofs. Reconstruction detects changed carrier classes,
retargeting and scalar activation; failed/uncomputed scalars remain non-Complete.
Explicit broad reads retain broad provenance. Existing-ID reads are immutable
during additive production but remain dependencies across reconstruction.

Actual commands below used `verification/scripts/low_artifact.py`, with debug
information and incremental compilation disabled, two build jobs, the separate
`target/bridge-architecture` target, and a 1 GiB reserve. Command/exit/resource
records are in `positive-population-review.json`; ignored raw logs remain under
`verification/generated/language-stability-bridge/`.

| Command | Result | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib specialization_witness -- --nocapture` | 5 passed, 0.37 s | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` | 62 passed, 14.04 s; 60,000-subject certificate 2,220,216 bytes | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib positioned_guard -- --nocapture` | 3 passed, 0.51 s | 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib proposal -- --nocapture` | 2 passed, 0.50 s | 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib input_action_body_closes_under_while_loop_with_nested_actions -- --nocapture` | 1 passed, Complete, 50.11 s | 0 |
| `git diff --check` | No output | 0 |

The strictly decreasing target-ID rule may retain additional ordinary implied
base relationships. This is an observable pre-acceptance graph-shape adjustment,
not a promise of minimal relationship count. Rule provenance, effective answers,
canonical standard target identities and the prohibition on inherited element
copies remain the acceptance obligations. After adoption of ADR 0026, future
observable graph-shape changes require its compatibility review/version process.
The historical accepted KerML publication is unchanged. Test-only Clippy cleanup
`6a31278` removes two redundant conversions after this verification, with no
production change; broader package/Clippy/Rustdoc checks belong to the closure
owner and root integration record.
