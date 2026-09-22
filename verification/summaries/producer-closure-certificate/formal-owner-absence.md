# Formal antecedent owner absence

`formal_constraint_applies` feeds formal specialization implications. Its
absent-owning-Type branch is an exhaustive negative premise, not an explicitly
current-graph-only result. It now merges an EffectiveOwnership closure read after
the required flag and owned-typing checks. The change touches only that absence
branch; present false flags and positive owner witnesses retain their behavior.

The regression starts with a composite Step in an unowned FeatureMembership and
later attaches that existing membership to a Behavior through an additive kernel
overlay. Without a certificate, or with a certificate whose Ownership producer
is pending, the negative antecedent is Incomplete. A closed absence is Complete
with exact certificate evidence and increments the certified-negative counter.
After attachment the same antecedent is positive and Complete without a
certificate; the earlier graph's certificate is rejected. An immutable false
composite flag still proves inapplicability without closure.

The cost is one typed dependency and certificate lookup for an absent owner.
Scheduler evaluation may revisit these negatives after its first stable witness.
No rule, authority profile, canonical source bytes or graph records are changed.

Verification at `fa77c52` used the low-artifact environment and isolated
`target/foundation-evidence` directory.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib formal_owner_absence` | 0 | 1 passed; 0.20 s |
| `cargo test --locked --offline -p agq-kerml-semantics --test formal_targets_v5 --test subobject_authority --test publication_v8_members` | 0 | 4 + 1 + 4 passed; no expectation changes |
| `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Clean |

No corpus candidate or publication acceptance is claimed.
