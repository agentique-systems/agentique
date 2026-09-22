# Negative query evidence

Implemented on fetched base `67fdddc81e5aceacea7cb407cdb2f45d621fd674`;
query changes `93bcb0f`, `521a794`, `4ae71dd`, and regression fixtures
`4e723e3`, composed with scheduler implementation `2801937`.

`SemanticContext` accepts only a scheduler-issued certificate for its exact
graph, producer registry and interpretation contract. Forks share immutable
evidence; changing a language interpretation or root availability discards it.
The optional closure contract uses `agq-producer-closure-context/1`; accepted
KerML Operational v9 and its `/26` receipt authority remain unchanged.

Formal owner-type and owned-typing negative antecedents require
`EffectiveTyping` evidence, including an empty effective type population.
Positive owner witnesses do not acquire a negative-closure requirement.
`SearchDependency::ProducerClosure` retains subject, requirement and certificate
digest (or an explicit missing witness). Explain uses this semantic proof
boundary, never a fabricated `FactKey`. Generic revision invalidation conservatively
invalidates a witness on any graph change. Provider discovery does not mistake
the certificate for another canonical graph producer.

Every certified check increments an evaluator-local atomic resource counter;
the counter changes neither certificate content nor semantic identity.

## Focused verification

Rust/Cargo 1.92.0 on Windows. Commands used `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`,
`CARGO_BUILD_JOBS=2` and the isolated target directory
`C:/Users/phili/github/agentique-systems/agentique/target/foundation-evidence`.
Disk preflight reported 51,406,036,992 bytes free before compilation.

| Actual command | Exit | Observed result |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --test producer_closure_evidence --test publication_v8_members --test formal_targets_v5` | 0 | 2 + 4 + 4 tests passed |
| `cargo test --locked --offline -p agq-kerml-semantics --lib context::extension_tests` | 0 | 5 passed, 64 filtered |
| `cargo test --locked --offline -p agq-kerml-semantics --lib closure_witness_is_invalidated` | 0 | 1 passed, 69 filtered |
| `cargo test --locked --offline -p agq-kerml-semantics` | 0 | Full package passed; 68 unit tests passed, 2 existing scale probes ignored; every integration binary and Rustdoc passed |
| `git diff --check` | 0 | No whitespace errors |

The authority matrix now separately observes uncertified current-graph
incompleteness and retains the effective `Complete` assertions after the actual
scheduler issues its certificate. The public integration tests cannot construct
certificates directly. They check negative owner evidence, typed Explain
premises, unchanged positive predicates, shared storage, resource accounting,
and rejection after graph, registry or interpretation changes.

Development failures were resolved before these passes: the old authority test
expected an uncertified exhaustive negative to be Complete, and a stale-graph
test initially created a Class without required scalar defaults. The latter now
adds an authored name to an existing valid record. No Systems publication was
run by this workstream.
