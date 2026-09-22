# Independent closure soundness review

Reviewed the integrated target-bound/reference-scalar changes at `a2320d1`,
the existing-scalar output audit fix `0eec834`, and the effective-property alias
matcher fix `9adb3e0`. This review does not establish Systems publication or
language readiness. No whole-library candidate was run.

Two concrete gaps were reproduced and fixed:

| Reproduction | Before fix | Required behavior after fix |
| --- | --- | --- |
| `cfc833e`: primitive scalar contribution to an existing Package under a Type-only producer target contract | Output audit accepted the out-of-contract write | Audit rejects Package and permits Classifier; existing-target scope and declared Scalar property are checked |
| `3c35da8`: a producer declares a primitive scalar base property while a query reads its effective redefined property | Pending write failed to invalidate the actual read | Resolve both property identities on the actual subject metaclass; the query read keeps EffectiveTyping unclosed until the writer closes |

The alias regression also audits and materializes an effective-property output
declared through its base property. Reference-valued and unknown scalar effects
retain conservative cross-subject handling. Registry schema 3 includes producer
target bounds and projection declarations. The SysML context independently
expects the complete combined producer registry; it does not obtain authority
from the certificate's claimed registry.

Commands used `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, and the isolated
`target/foundation-evidence` target.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib semantic_target_bound` after `0eec834` | 0 | 3 passed |
| `cargo test --locked --offline -p agq-kerml-semantics --lib reference_scalar_writes_reopen_cross_subject_queries_and_requirement_masks` after `0eec834` | 0 | 1 passed |
| `cargo test --locked --offline -p agq-kerml-semantics --lib scalar_producer_base_property_matches_effective_read_alias` after `9adb3e0` | 0 | 1 passed, 110 filtered out; 0.08 s test runtime |

The Actions result-ordering patch `153bbeb` was reviewed without changes. It
skips the extra owned-feature suppression population only when there are no
inherited results or each inherited result is already the target of a known
owned-result implication. Other cases retain the full suppression scan. Typed
return-membership, conjugation and feature-chain reads retain their ordering and
canonical support. No concrete ordering or completeness defect was found.

A read-only Python ZIP/qualified-root scan of the original Systems KPAR found
all 21 documents and SHA-256
`df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f`.
The selected Actions8 and Medium12 scopes in `actions-preflight.md` have no
missing source document or explicit Systems-qualified root outside the selected
scope. The scan exited 0. This establishes textual dependency coverage only;
producer-implied dependencies and semantic closure still require the scoped
candidate audits.

Trusted Systems restoration remains disabled and unimplemented until actual
publication acceptance. The approved finite-catalog authority and exact-graph
restore design is recorded in
`../sysml-systems-acceptance/trusted-restore-preparation.md`. Current export bytes
do not authorize caller-issued closure certificates or accepted facades. The
accepted KerML receipt path is unchanged.
