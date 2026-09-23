# Monolithic scheduler observations

This change preserves the producer registry, dependency scopes, fixed-point
algorithm, proof/search transport and publication gates. It adds bounded
per-frontier observations and corrects when completed-frontier evidence is
reported. All producer families in a subject still run against the same immutable
graph. No new effect registry or source/component partition is introduced.

Previously `PublicationStage` was emitted before certificate issuance. Its
certificate counts and cumulative build time described an older frontier. Changed
frontiers also failed to refresh their closure counts. Progress now emits only
after the frontier's certificate has been issued; the watchdog receives actual
`frontier=N` and `closed_pairs=N` values. A regression checks the final observer
against the final certificate, including requirement counts and incomplete pairs.

Each frontier reports evaluated, applicability-skipped and reevaluated subjects,
planned and accepted elements, accepted occurrences, next dirty subjects, family
attempts, and separate planning/query-cache, dependency-index, materialization,
certificate invalidation and certificate construction times. Family timings
measure their existing blocks. FeatureValue/FeatureValuation share query work;
their inclusive shared time is explicit and must not be added twice. Extension
callback time is likewise shared among its actually recorded family evaluations.
Timing is observational and never participates in semantic identity or acceptance.

Dirty reasons use seven compact categories. Graph and provider reasons describe
the existing conservative invalidation keys, not a proof that each query answer
changed. Incoming-key invalidation reports possible changed relationship endpoints
and provider searches. The index intentionally coalesces element fact and element
population reads; `GraphFactChanged` therefore also covers those provider reads.
Family reason counts describe the subject's scheduling trigger because the
existing scheduler evaluates families together. No subject/read trace or duplicate
effect registry is retained. Unchanged authoritative query/evidence contracts
remain the basis for future finer-grained invalidation.

## Evidence and interpretation

[The profile](scheduler-profile.json) retains the prior medium stages 15–32 and
new focused contextual shapes. The historical certificate time near 100 seconds
was cumulative: consecutive late-stage observations differ by 2.924–5.255 seconds.
Those old logs cannot rank producer families. They also cannot assign a build
delta to the same logged stage because of the observer placement above.

The permanent directed/undirected Function result, FeatureReference and
FeatureValue shapes both converge Complete with fully closed certificates in
four frontiers, two contextual. In these debug runs FeatureReference's two
attempts cost 7.273/8.070 ms; shared valuation/value work costs 5.823/5.393 ms;
positional redefinition costs 5.014/5.602 ms. These are focused shape measurements,
not estimates of the real medium tail. The integrated medium acceptance run must
collect actual family timings for rounds >=15 before making a corpus-tail claim.

## Existing scope review

The implementation and permanent counterexamples already constrain these writers:

| Family | Existing contract and retained guard |
| --- | --- |
| FeatureValue | Subject scope; contextual stratum; binding and chain relationship classes; complete featuring domain required. Valuation structural evidence remains separate. |
| FeatureReference | Subject scope; contextual stratum; actual referent/raw-result/domain reads; fresh binding infrastructure cannot change declared referent identity. |
| Expression/Function result | Subject scope; explicit relationship classes and fresh end population; no FeatureValue carriers or retyping of the producer subject. |
| Variable snapshot | Subject plus owning Type; featuring writes restricted to the variable; membership/redefinition outputs retain owner and negative snapshot-population reads. |
| Feature-chain result | Subject plus selected owned Features; direct result/source-target selector; existing first-input membership writes remain declared. Nested input-expression result is excluded by regression. |
| Index/Select result | Subject plus owned results for operational producer profiles; Array guard and result evidence remain required. |

No additional scope restriction was justified by the focused evidence. Existing
conservative reads remain; speculative narrowing would violate this milestone's
test-first requirement. The worklist/full-scan equivalence suite continues to
compare actual graph records, proofs/searches, query answers and certificates.

Commands, actual exit codes and retained output digests are in
[scheduler-commands.json](scheduler-commands.json). No full Systems attempt or
Systems SCC planning run was performed in this workstream.
