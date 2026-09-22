# Publication acceptance review

Two acceptance defects were corrected before the publication attempt:

- Construction preflight and strict final acceptance now use the same authority
  audit of the last producer stage. A resolved earlier target failure cannot
  reject a completed final graph; unresolved final findings still reject it.
- The exact 21-document corpus must contain 1,327 mandatory source assertions.
  Acceptance cannot silently degrade to a smaller N/N population. Duplicate
  complete requests, including source revision/node/range, cannot pad the count;
  distinct source assertions sharing a relationship/property remain distinct.

The existing source-provenance fixture now checks Operational v2 and verifies the
actual 1,327 distinct assertions from all original document bytes. It performs
parsing and declared lowering only, with no producer replay/publication candidate.

Read-only review also confirmed that final SysML bindings are independently
validated before and after strict closure; producers resolve current paths and
check bound IDs. Construction certificates are discarded before strict closure;
the final scheduler includes generated subjects and issues fresh evidence under
the final binding contract. The export receipt records source/KPAR, profile,
grammar/corrections, descriptors, accepted KerML, bindings, graph and closure
identities. Exported bytes do not authorize trusted Systems restoration.

Low-artifact isolated verification, all exit 0:

- `cargo test --locked --offline -p agq-kerml-text --lib sysml::publication::tests`:
  6 focused publication tests passed.
- `cargo clippy --locked --offline -p agq-kerml-text --lib -- -D warnings`:
  no warnings.
- After adding the real-corpus reference-population check:
  `cargo test --locked --offline -p agq-kerml-text --lib all_systems_declared_fact_sources_match_exact_operational_syntax_nodes`:
  1 test passed, including exact Operational v2 source/node provenance and all
  1,327 distinct mandatory assertions.

These focused results do not constitute Systems publication acceptance or
language-foundation readiness.
