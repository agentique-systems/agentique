# Producer closure bridge

Implemented initial zero-writer evidence, dependency-aware revalidation between
additive frontiers, and compact `ProducerClosureCheckpoint` transport across
reconstructed graphs without retaining old model indexes. Checkpoints compare
actual record, endpoint, pending-input, proof-search and output-support identities.
New producer opportunities reopen causal closure even if prior values are equal.

Explain distinguishes accepted immutable dependencies from local producer closure.
Subject-scoped local producers cannot reopen protected dependency populations;
external source/inverse carriers remain open when a registered writer can change
them. An arbitrary immutable overlay is not accepted publication authority.
Absent ownership is Complete only with closed ownership and complete navigation.
The independently versioned optional certificate/context contract is now version 2;
accepted historical KerML publication/profile/receipt identities are unchanged.

Focused verification (Windows, low-disk profile, 2026-09-22):

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo check --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics` | Initial compile identified missing private dependency field in accepted-query constructor; corrected before tests | 1 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure --no-run` | Test binary compiled | 0 |
| `cargo fmt --all` | No output | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` | 47 passed, 0 failed; 10.73 seconds | 0 |

The 60,000-subject/80-family proof remains 2,220,216 bytes (300,000
applicable pairs, 299,995 closed, one incomplete; 6,305 ms issuance). Optional
revalidation metadata is accounted separately from the compact receipt proof.

Environment: `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`; target directory
`target/bridge-closure`. Package-wide tests, Clippy and strict Rustdoc follow at
integration. These checks do not establish Systems publication acceptance.


Provider review identified and corrected a separate zero-writer gap: unresolved
relationship source endpoints, known-source missing targets, pending specialization
scopes and unavailable namespace populations remain semantic provider obligations.
Required FeatureTyping, Redefinition, Subsetting, FeatureChaining and Membership
carriers are covered generically, including namespace providers with no kernel
lower-bound obligation. External ordinary source endpoints may affect dependency
subjects; unknown composite owners cannot adopt protected dependency elements.

Additional outcomes:

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics` | KerML unit/integration suite passed; SysML49 passed/5 failed: legacy synthetic immutable fixtures lacked actual dependency closure authority | 101 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib initial_closure_ -- --nocapture` | All3 adversarial provider regressions passed after generic fix | 0 |

Raw package output is ignored at
`verification/generated/language-stability-bridge/closure-package-tests.log`.
Synthetic fixture correction remains separate from production acceptance.

Follow-up provider review closes causal transport and delayed carrier ownership:
retained producers reading an opened requirement are reopened transitively, and
namespace attachment of an existing owning relationship reopens its descendants.
The reviewed regressions are retained as permanent focused tests.

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` | 53 passed, 0 failed; 10.23 seconds. Compact 60,000-subject proof: 2,220,216 bytes, 6,082 ms issuance | 0 |
