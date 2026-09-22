# Scheduler-issued producer closure

Implementation source: `9adb3e0` on the isolated
`foundation/producer-closure` branch. This summary records generic closure work;
it does not establish Systems Library acceptance.

Families have stable string identities, explicit potential effects, applicability,
minimum strata and write scopes. Existing-subject effects differ from effects on
fresh semantic subjects: a new relationship pointing to an existing source is
not a fresh-source exemption. Registry identity covers the sorted descriptors and
the explicit requirement/effect mapping. Declarations are trusted implementation
contracts; batch output checks provide defense in depth, not inferred declarations.
Optional exact relationship-class and positional member-population bounds refine
typed reads; absent bounds remain conservative. Future scalar producers reopen
parameter/end exclusions even before an applicable subject exists. Property
redefinitions are resolved through the registry; unknown scalar properties remain
conservative. Pending ownership changes expand ownership-dependent write scopes.
Existing semantic target metaclass bounds distinguish a Type from its membership
carriers; fresh semantic subjects retain their separate effect contract. Primitive
scalar reads remain precise, while reference-valued or unclassified scalar effects
conservatively reopen cross-subject requirements and inverse searches. Registry
schema 3 versions this interpretation. Existing scalar contributions are checked
against declared property, target class and write scope.
Primitive scalar declarations and reads resolve both the base and effective
property identity on the actual subject; the regression covers query reads,
closure blocking, accepted output audit and canonical materialization.

The scheduler retains per-family evaluation status and precise query reads.
Incomplete or dirty families prevent closure of dependent complete evaluations,
including scalar-to-typing activation, cross-source inverse searches and families
that can first become applicable on future generated subjects. Pending creators
conservatively account for future families that can write existing subjects.
Certificates are issued only after the frontier is quiescent. Subject states use
two bits per family; query requirement coverage uses one byte per subject.

Certificates are private immutable values bound to the exact semantic graph,
registry and context contract. The context holds a shared `Arc`. Negative proof
queries retain typed certificate evidence. Canonical searches retain only the
reopenable subject/requirement dependency, so historical certificate identity does
not contaminate canonical graph identity. A populated overlay can be reevaluated
under a new interpretation contract without reconstructing its graph.

Read precision preserves canonical proof facts while distinguishing filtered
relationship populations, fixed stored scalar facts, protected dependency storage
and searches open to local relationship additions. An immutable standard record
does not imply all semantic queries about it are closed: external source/inverse
searches remain dependencies. Current non-typing requirements conservatively block
across relevant unfinished effects. `ValueContext` includes connector structure.

Verification used Windows, Rust/Cargo 1.92.0, isolated `target/foundation-closure`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`.

| Actual command | Result |
| --- | --- |
| `cargo test -p agq-kerml-semantics --lib producer_closure_tests -- --nocapture` at `9adb3e0` | Exit 0; 34 passed |
| `cargo test -p agq-kerml-semantics --lib -- --nocapture` at `e039114` | Exit 0; 93 passed, 2 existing scale probes ignored |
| `cargo fmt --all -- --check` at `9adb3e0` | Exit 0 |
| `cargo clippy -p agq-kerml-semantics --all-targets --locked --offline -- -D warnings` at `9adb3e0` | Exit 0 |
| `RUSTDOCFLAGS=-D warnings cargo doc -p agq-kerml-semantics --no-deps --locked --offline` at `0eec834` | Exit 0 |

The 34-test matrix covers delayed producer activation; incomplete/inapplicable
families; registry and graph mismatch; malformed registration; FIFO, LIFO,
reversed, partitioned and reference-scan scheduling; varied batches; actual
negative-proof-consuming output determinism; existing-overlay revalidation;
ownership-derived chain/reference sources; transitive scalar dependencies;
arbitrary inverse reads; fresh-source/reownership misuse; subtype populations;
future cross-subject producers; protected stored reads versus external semantic
searches; existing scalar versus absent/collection reads; current/future bounded
relationship classes; strict descendant scopes; pending ownership attachment;
future scalar projection activation including property aliases; malformed or
qualifying membership outputs that violate an empty positional bound; immutable
records with mutable noncomposite inverse navigation; reference scalar writes;
current/future target-class bounds; existing scalar output target audits; and
effective scalar property aliases.

The synthetic scale fixture has 60,000 subjects and 80 registered families, with
300,000 applicable evaluations carrying nonempty reads. One incomplete writer
blocks four otherwise complete readers: 299,995 closed pairs remain. Certificate
storage is **2,220,208 bytes**; the final focused debug run constructed it in
**8,049 ms** during concurrent worktree activity (the earlier 32-test run measured
6,186 ms). This measures the certificate,
not whole-publication runtime or peak scheduler memory.

Development failures exposed conservative read conflation and obsolete fixture
assumptions about already stored defaults. The resulting regressions preserve
open absence/collection searches instead of treating a fixed point as evidence.
No full Systems candidate was run by this workstream.
