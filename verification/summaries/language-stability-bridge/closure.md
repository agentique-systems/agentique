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

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` | Finished, no warnings | 0 |
| `cargo clippy --locked --offline -p agq-sysml-semantics --all-targets -- -D warnings` | Finished, no warnings | 0 |
| `RUSTDOCFLAGS=-D warnings cargo doc --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics --no-deps` | Both public semantic APIs documented, no warnings | 0 |

Synthetic combined-producer fixtures now close their standalone dependency using
all actual anchor roles and explicit portion classification on skeletal Usage
anchors. They mount that exact graph with `ProducerClosedDependency`, preserving
shared Arc identity; a fabricated naming digest/immutable pointer is not authority.
Final query contexts include the same authenticated dependency contract.

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-sysml-semantics --lib actions_micro_closes -- --nocapture` | Initial adaptation caught Arc type/error conversion/receiver lifetime mistakes; corrected locally; focused test then passed | 101, then 0 |
| `cargo test --locked --offline -p agq-sysml-semantics` | 60 passed, 0 failed, 33.15 seconds; doc tests passed | 0 |
| `cargo fmt --all -- --check` | No output | 0 |
| `cargo clippy --locked --offline -p agq-sysml-semantics --all-targets -- -D warnings` | Finished, no warnings after witness fixture correction | 0 |

Declared-source target lookup follow-up (2026-09-23): a pending membership writer
was reopening a completed producer that read only original source ownership.
`StructuralSearch::DeclaredProperty` now distinguishes the original submitted slot
(including absence) from the current derived aggregate. It survives proof transport,
ignores additive derived ownership/adoption, and remains a source-reconstruction
read. SysML anchor lookup requires these original ownership edges and retains
endpoint, origin, visibility and name evidence. Pending source scopes/endpoints
remain incomplete. Mixed current/declared reads remain broad, in either order,
including persisted current facts without an explicit current-property search.
The SysML query contract is version 5; accepted KerML rule-set version 26 is unchanged.

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib declared_source_population -- --nocapture` | 3 passed; initial test setup omitted required membership visibility, corrected; mixed-read/reconstruction/provider tests pass | 101, then 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib standard_anchor_path -- --nocapture` | 1 passed; derived adoption excluded, original source edit exposes ambiguity | 0 |
| `cargo test --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics` | Kernel and KerML suites passed; SysML60 passed/1 failed because old test expected broad NamespaceMembers instead of precise declared-source evidence | 101 |
| `cargo test --locked --offline -p agq-sysml-semantics` | After updating that evidence assertion, 61 passed/0 failed, 33.16 seconds; doc tests passed | 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib context_identity -- --nocapture` | Version5 context: 3 passed | 0 |
| `cargo clippy --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` | Finished, no warnings | 0 |
| `RUSTDOCFLAGS=-D warnings cargo doc --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics --no-deps` | Three crates documented without warnings | 0 |
| `cargo fmt --all -- --check` | No output | 0 |

The diagnostic-only untyped combined synthetic library still exposes broad KerML
binding-population reads because it lacks the real accepted KerML dependency
boundary. This experiment was removed; it does not justify changing the accepted
KerML contract. The focused declared-source regression independently verifies the
SysML correction. No corpus publication ran in this worktree.
