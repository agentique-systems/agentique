# Query footprint precision

2026-09-22, Windows, low-artifact environment (`CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`).
Isolated target: `target/foundation-actions`.

The query changes preserve canonical backing facts while exposing the relationship
class or ordered feature projection actually observed. FeatureValue producers first
inspect their value population; empty populations do not read direction or
specializations. Positional queries retain canonical ordering and inherited IDs.

SysML 2.0 `ActionUsage::inputParameters` explicitly selects `f.owner = self`
(`standards/normative/sysml-2.0/SysML.xmi`, operation at line 3644). Transition
payload construction must therefore use owned inputs, preserving inherited
parameter identities. Extra pending ancestry affects redundancy checking, not the
independent implication from known trigger and input parameters.

Completed checks:

| Command | Result |
| --- | --- |
| `cargo test -p agq-kerml-semantics --lib feature_values -- --nocapture` | Exit 0; 2 passed, including the empty-value read regression |
| `cargo test -p agq-kerml-semantics --test positional_publication --test redefinition_v2 --test redefinition_end_v4 --test ordered_results -- --nocapture` | Exit 101; ordered 6, positional 2, end 2 passed; redefinition 9 passed and one feature-chain regression failed |
| `cargo test -p agq-kerml-semantics --test redefinition_v2 conjugation_and_feature_chain_supply_original_member_identities` | Exit 0 after fixing the missing separate FeatureChaining population; 1 passed |
| `cargo clippy -p agq-kerml-semantics --all-targets -- -D warnings` | Exit 0 before the final feature projection change |
| `cargo test -p agq-kerml-semantics --test ordered_results` | Exit 0; 7 passed, including conditional inherited-result suppression reads |
| `cargo test -p agq-sysml-semantics` | Exit 0; 51 passed and doc tests passed after expression membership precision |
| `cargo test -p agq-kerml-semantics --lib variable` | Exit 0; 4 passed after scalar-write target audit |
| `cargo test -p agq-kerml-semantics --lib feature_values` | Exit 0; 2 passed after scalar-write target audit |
| `cargo test -p agq-kerml-semantics --test initial_values_v10 --test ordered_results --test positional_publication` | Exit 0; 1 + 7 + 3 passed after scalar-write target audit |
| `cargo test -p agq-sysml-semantics` | Exit 0; 51 passed again after scalar-write target audit |
| `cargo fmt --all -- --check` | Exit 0 |
| `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` | Exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p agq-kerml-semantics -p agq-sysml-semantics` | Exit 0 |

The failed feature-chain test exposed a real implementation defect in the earlier
class-filtered lookup. It was fixed in `e0a5f7d`; the assertion was retained.
The combined Actions micro, root Usage without an owner, and shared variable
snapshot/value-context fixtures all pass. Result producers select their exact
membership classes. Result queries retain broad owned-feature suppression evidence
when inherited results can survive, and avoid those unrelated reads when known
positional redefinitions already suppress every inherited result.

No whole Systems Library candidate was run from this workstream. These fixture
results do not establish Systems publication acceptance.

Final primitive scalar-alias matcher (`9adb3e0`, local cherry-pick `118bfed`):
the same low-artifact environment reran `cargo test -p agq-sysml-semantics`
(51 passed), `cargo test -p agq-kerml-semantics --lib variable` (4 passed),
`cargo test -p agq-kerml-semantics --lib feature_values` (2 passed), and
`cargo test -p agq-kerml-semantics --test initial_values_v10` (1 passed).
Every command exited 0; all three combined SysML closure fixtures remain green.
