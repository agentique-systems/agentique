# Accepted effective-core gate preparation

This is test preparation on source `f42a97c68326a43bf56ed15e037e1163aec6a79a`,
not a passing phase M result. No receipt, profile default, publication catalogue
or standard artifact is changed. ADR 0026 remains proposed.

The existing accepted self-model gate exercises the five real SysML documents,
architecture dependencies, complete references and producer closure, independent
programmatic equivalence, case and View/Metadata fixtures, three immutable
revisions, shared accepted standards and concurrent readers. The shared
`rich_summary` now also calls `effective_occurrence_definitions(workspace)` and
`effective_connection_definitions(queryConnection)`. Both must be Complete and
retain the original authored `ProjectWorkspace` and `SemanticAccess` identities,
respectively. The existing comparison preserves the entire returned population,
including canonical dependency identities, in both independently built models.

These assertions require no new source declarations or programmatic model edits.
They do not establish the separate plain-Association interpretation: that has
the profile/domain regression tests and the exact Systems corpus witnesses.

## Phase M execution after actual acceptance

First activate only the actually issued v3 receipt and bindings. Update the held
test's explicit v2 receipt and restoration call to that accepted interpretation;
make the same profile choice in both positive and tampering helpers of
`accepted_systems_cache.rs`. Retain historical profile behavior and the enum's
Published default. Assert the restored dependency has the expected v3 identity.

Set `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE` to the original accepted
cache artifacts. Run the heavy gates sequentially and retain their actual output,
exit codes, source identities and artifact hashes:

```text
cargo test --locked --offline -p agq-kerml-text --test accepted_systems_cache accepted_systems_cache_roundtrip_and_tampering -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline -p agq-kerml-text --lib sysml::tests::accepted_self_model::accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline -p agq-kerml-text --lib agentique_ -- --test-threads=1
cargo test --locked --offline -p agq-sysml-semantics --lib
```

The final workspace test run may supply the ordinary package-test evidence if
it tests the same source. It does not execute either ignored acceptance gate.
The self-model gate is the accepted effective-core harness; the old
`sysml_foundation` example explicitly constructs a Published candidate and reports
incomplete Systems foundation, so it cannot substitute for this gate.

The cache test requires 1,327 Complete references, 21 documents, all closure
requirements, exact graph/context/bindings/selected-evidence restoration, zero
producer replay counters and tamper rejection. The self-model test requires
Complete effective answers over the actual shared dependency. Its new comparisons
cover the two newly exposed typed APIs. The separate focused and real-corpus gates
cover enumeration literals, inherited BinaryInterface ends, return/end cycles and
the bounded HappensDuring case; these are not authored self-model declarations.

Record the measured readiness result under
`verification/summaries/final-language-acceptance/` before adopting ADR 0026.
ADR 0027 remains proposed. Phase N is separate: this source-project test is not
`agq-modeling-workspace` integration, Working/Validated revision acceptance, or
the 100-document/five-revision/four-reader scale gate.

## Checks actually run for this preparation

Only formatting and diff checks ran. No Rust compilation, cache restoration,
semantic gate, corpus execution or shared-target build ran.

| Command | Exit | Output |
| --- | --- | --- |
| `rustfmt --edition 2024 --check crates/kerml-text/src/accepted_self_model_tests.rs` | 0 | empty |
| `git diff --check` | 0 | empty |

The checked test-file SHA-256 is
`496cc0255e2636437f28354b8dd6c143134ac80f3b6f4c8415f5155803907b28`.
Both empty outputs have SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
