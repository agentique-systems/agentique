# Gen1 / Gen2 architecture audit

The [disposition](../../../docs/gen1-gen2-architecture-audit.md) inspects all ten
requested implementation areas at fetched main
`0fbdf3ee19788a260b99d89e71dbd28f432d8243`, including the source implementations,
not only package names. No Gen1 application or release obligations are changed.

The existing semantic stores and language frontends are replaced for Gen2;
transaction/authority, progress, SQLite reliability, jobs, transport and paging
patterns can be migrated or adapted above the language boundary. SourceProject
is useful Gen2 source-history infrastructure but currently rejects recovered
mixed syntax and does not attach an accepted Systems publication.

Reproduce the dependency check and its adversarial tests without compiling Rust:

```text
python verification/scripts/generation_boundary.py
python -m unittest discover -s verification/scripts -p test_generation_boundary.py
```

The check includes every declared normal/build/development, optional and
target-specific workspace edge, renamed packages and transitive workspace paths.
It also protects a future additive modeling-workspace crate and the kernel's
language independence. Third-party dependency auditing is outside its scope.
The source inspector and Cargo checker remain outside semantic-model tests.

Observed result: 19 workspace packages, eight protected packages, zero forbidden
dependency paths. All eight adversarial tests passed, including renamed optional
target dependencies, build/dev edges, transitive helpers, cycles and empty-input
rejection. Both commands and `git diff --cached --check` exited 0. The metadata
subprocess used Cargo `1.92.0 (344c4567c 2025-10-21)`; Python was `3.12.10`.
No Rust source or manifests changed in this audit, and no workspace/runtime
acceptance is inferred from these checks.

Actual command arguments, tool versions, tested source/patch identity, exit codes
and output digests are recorded in `summary.json`. Raw outputs are in ignored
`verification/generated/gen1-gen2-audit/`, following ADR 0021.
