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

The failed feature-chain test exposed a real implementation defect in the earlier
class-filtered lookup. It was fixed in `e0a5f7d`; the assertion was retained.
The combined Actions micro fixture remains the gate for the final projection
contract. No whole Systems Library candidate was run from this workstream.
