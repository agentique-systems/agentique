# Effective structural query stage

The SysML query contract is now `agq-sysml-query/4`. Effective APIs preserve
scheduler closure searches and current canonical identities. Current naming has
explicit `current_names`/`current_qualified_name` APIs; effective naming requires
closure. Generic family selectors cover all 13 platform Usage families, alongside
typing, composite nesting, connection/interface ends, action/state membership
roles, requirement/case roles and return parameters. See
[the API contract](../../../docs/sysml-effective-structural-queries.md).

The actual combined scheduler Actions micro fixture establishes Complete
effective usages, trigger and payload identities. An additional variation witness
remains Incomplete after producer closure. Unsupported modifiers are not erased
by scheduler quiescence. Additional tests check empty-population negative evidence,
ordered ends, type domains, invalid subject classes, inherited identities and
unchanged old-revision answers after composition/name changes.

The isolated branch was tested on source base
`8c172b55b76a7f494a8ef18abb9ecac563912563`. Toolchain:
`rustc 1.92.0 (ded5c06cf 2025-12-08)`,
`cargo 1.92.0 (344c4567c 2025-10-21)`. Environment is the low-disk profile in
the adjacent README. The normalized source-set SHA-256 is
`f56b9412bcfbbd22440d3c6bb26f04e57ceccb63087cac9e2e8b39a8817c210a`,
over sorted `crates/sysml-semantics/src/*.rs`, the two self-model test files
and `examples/sysml_foundation.rs` (path, NUL, LF-normalized bytes, NUL).

| Actual command | Exit | Output |
| --- | --- | --- |
| `cargo test -p agq-sysml-semantics --lib` | 0 | 58 passed |
| `cargo test -p agq-sysml-semantics connector_and_interface_end_order --lib` | 0 | 1 additional test passed |
| `cargo test -p agq-kerml-text --lib` | 0 | 36 passed |
| `cargo clippy -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` | 0 | Finished dev profile |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p agq-sysml-semantics --no-deps` | 0 | Documentation generated |
| `cargo fmt --all -- --check` | 0 | No formatting differences |

This stage did not publish Systems or consume an accepted Systems library.
Integration must rerun the Actions fixtures after the independently developed
immutable-dependency closure changes; their synthetic anchors have no production
acceptance receipt. Final readiness still requires accepted-publication ingestion
and closed authored self-model queries.
