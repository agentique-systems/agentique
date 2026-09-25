# Publication interpretation-input review

The first standards check returned 1 because three recorded interpretation-file
hashes changed: `Cargo.lock`, `crates/kerml-text/src/source_inputs.rs` and
`crates/kerml-text/src/sysml/source.rs`. The original failure and exact output
hash remain in `commands.json`.

The lockfile adds the runtime distribution crate and Studio's dependency on it.
It changes no external package version, checksum or dependency graph. The two
language implementation files add optional observational elapsed-time recording
to the existing authored compilation path. Construction, refinement, scheduler,
certificate, reference, effective audit and invalidation calls remain in place;
their inputs and acceptance decisions are unchanged. Timings are separate from
semantic identities, durable checkpoints and semantic cache payloads. The direct
source lowering path passes no timing collector. No effective audit reuse or
semantic shortcut is introduced.

The runtime adapter depends inward on the accepted language facades. It cannot
issue a publication and is prohibited from becoming a language/workspace/kernel
dependency by the generation-boundary check. Original library bytes, grammar and
semantic manifests, descriptors, producer registries, bindings, accepted receipts
and the trusted catalogue remain unchanged.

This review permits refreshing only the non-authoritative
`standards/sysml-publication-inputs.json` inventory. Its
`grants_publication_authority: false` remains explicit. Capture is not publication
rebuilding, runtime authentication, semantic performance acceptance or evidence
that the missing accepted cache bytes exist.
