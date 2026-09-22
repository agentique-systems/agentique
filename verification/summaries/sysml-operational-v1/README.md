# SysML Operational v1 grammar

The exact 21-document Systems Library now parses completely under the explicit
`agentique-sysml-2.0-operational/1` syntax profile: zero recovery and 86,538 original
source bytes preserved. The Published profile retains 13 complete documents and
the eight exact historical failures, including diagnostic byte ranges and tokens.

The [compatibility manifest](../../../standards/grammar/sysml-2.0-operational-v1.json)
freezes the four authorized decisions as five exact production RHS changes:
AllocationDefinition dispatch, Case return members, the two reviewed end-usage
prefix changes, and optional `assert`/`not` in SatisfyRequirementUsage. It pins the
published PDF, maintained grammar and original KPAR; those inputs are unchanged.
No generic body or optional-end fallback was introduced.

`parse_sysml` and `parse_with_dialect(Dialect::SysMl, ...)` retain Published behavior.
`parse_sysml_with_profile(SysmlSyntaxProfile::OperationalV1, ...)` selects the
operational tables. A Document retains that profile across edits. Operational
syntax has a separate identity domain; existing KerML and Published identities
are unchanged. Both profiles use the existing shared lexer, production arena,
source provenance and expression precedence.

The shared vocabulary remains 535 production kinds. Published SysML has 2,032
grammar alternatives and Operational v1 has 2,040. The original KerML and strict
SysML generated tables are unchanged. The generator verifies both profiles and
the exact manifest before accepting checked-in tables.

[grammar-status.json](grammar-status.json) records every path/profile outcome,
root production, production-node count, recovery count and byte preservation.
[grammar-verification.json](grammar-verification.json) records the implementation
commit, command arguments, exit codes, tools and concise actual outputs. Fifteen
Rust tests and eight Python tests pass; package Clippy, strict Rustdoc, formatting
and grammar stale checks pass. Raw output is ignored under `verification/generated/`.

Reproduce the corpus report with:

```text
cargo run --locked --offline -p agq-kerml-syntax --example sysml_corpus
```

This evidence establishes textual recognition. Canonical lowering, semantic
authority, combined producer closure and Systems publication have separate gates.
