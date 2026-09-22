# Owned relationship exclusion evidence

`OwnedRelationshipsExcluding { owner, class, excluded }` records the direct
owned relationship population in the selected metaclass and its registered
subtypes, excluding each named subtype family. The immutable descriptor registry
defines every class identity. Empty exclusions preserve the selected population;
unknown descriptors cannot justify a negative result.

The query evidence transports the complete sorted exclusion set through a
language-neutral kernel search variant. Canonical semantic encoding appends tag
11 without changing existing tags or accepted KerML receipt encoding. Ordinary
cache invalidation retains owner changes and context/descriptor changes. The
producer matcher is implemented separately; until integrated, extraction treats
this population conservatively as a global read.

Verification at API commit `de69dd0` plus these tests used the established
low-artifact environment and isolated `target/foundation-evidence` directory.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo check --locked --offline -p agq-kerml-semantics` | 0 | Clean |
| `cargo test --locked --offline -p agq-kerml-semantics --lib owned_relationship_exclusions` | 0 | 2 passed; 0.06 s |
| `cargo test --locked --offline -p agq-kernel --test archive` | 0 | 6 passed; 0.03 s |
| `cargo clippy --locked --offline -p agq-kernel -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Clean |
| `cargo fmt --all -- --check` | 0 | Clean |

The regressions cover exact native/persisted evidence, empty and unknown
exclusions, owner and descriptor-context invalidation, archive roundtrip and
sharing, canonical digest distinction for owner/class/exclusions, and insertion
order independence. No corpus candidate or publication acceptance is claimed.
