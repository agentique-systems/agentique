# Transactional Systems artifact issuance

Implementation base: `0c4c0adae19125c5a72b3e3521c7df1c6c6d84f0`.
Combined Rust verification tree: `4c0a455` (issuance `bb30b21` plus finalization
API `c67bacd`; only this evidence paragraph changed afterwards).
This implementation evidence does not claim Systems acceptance or activate the
compiled receipt catalogue. The lead publication run records the final result.

`sysml_systems_publication --cache=<accepted KerML cache>
--finalize-converged=<retained journal> --resume-sha256=<independent SHA-256>
--output=verification/generated/<fresh bundle>/report.json` takes the finalizer
branch before all preparation and producer calls. It rejects slice/scheduler
options. The report separates zero finalization evaluations from retained
scheduler counters and records the authenticated journal entry and live closure
identities.

The output's parent must be a fresh directory. Candidate cache, receipt,
bindings and report remain in a sibling `.candidate-*` directory until a single
same-volume rename exposes the entire bundle. Existing bundles are never
replaced. Read-only audit observations are flushed and synced in a separate
`.audits-*` sibling, including on failures. Abandoned candidate directories confer
no trusted restoration authority.

Before promotion, the live accepted facade independently regenerates all cache
entry digests and identity documents. Caller receipt/binding documents must match
exactly. Consuming candidate verification drops the original Systems graph before
restoring its replacement, retaining the exact immutable KerML dependency and
scheduler certificate. Restoration reconstructs descriptors and all binding roles,
authenticates decoded graph/proof/search bytes, binds the certificate to the
actual restored context, and compares semantic/publication identities. The
ordinary cache-restoration API still requires compiled independent authority.

`npm run standards:check` now includes a fail-closed Systems publication stale
guard after acceptance. Receipt, bindings and `standards/sysml-publication-inputs.json`
must exist together. It cross-checks source/library/profile, semantic, descriptor,
registry, certificate and binding identities, plus a conservative inventory of
Gen2 interpretation sources and manifests. Dedicated test files and workspace
implementation/docs are excluded. Line endings are normalized for source pins;
original KPAR bytes remain checked by the existing artifact gate.

After accepted publication and semantic regression verification, the explicit
maintenance command `node tools/sysml-publication-stale.mjs --capture` records
freshness. This command does not run automatically or issue acceptance authority.
A source change requires review and relevant semantic regression acceptance before
refreshing those observations. This does not freeze implementation details.

Focused checks (Rust `1.92.0`, Node `v22.11.0`, Python `3.12.10`):

| Command | Exit | Result |
| --- | --- | --- |
| `node --test tools/sysml-publication-stale.test.mjs tools/sysml-artifacts.test.mjs` | 0 | 8 passed; includes changed source/descriptor/registry/semantic/binding and partial-installation rejection |
| `python -m unittest discover -s verification/scripts -p test_systems_publication_gate.py` | 0 | 4 passed; strict artifact gate now requires candidate restoration and atomic promotion evidence |
| `cargo fmt --all -- --check` | 0 | Passed |
| `cargo check --locked --offline -p agq-kerml-text --example sysml_systems_publication` | 0 | Combined finalizer/issuance compile passed (3.72 s) |
| `cargo test --locked --offline -p agq-kerml-text --example sysml_systems_publication` | 0 | 3 passed: interruption invisibility, complete bundle promotion, destination collision preservation |
| `cargo clippy --locked --offline -p agq-kerml-text --all-targets -- -D warnings` | 0 | Passed (6.77 s) |
| `cargo test --locked --offline -p agq-kerml-text --lib sysml::publication::cache::restoration::tests` | 0 | 4 passed: schema, archive bounds/inventory and stale source rejection |

Initial combined Node execution failed because this isolated worktree had no
`fflate` package (exit 1). A local ignored junction to the existing installed
package resolved that environment issue; the command above then passed. Raw
focused logs are ignored under `verification/generated/systems-finalization-issuance/`.
Rust checks used the shared ignored `target/platform-finalization` directory with
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`
and `CARGO_BUILD_JOBS=2`. No publication finalizer or producer scheduler ran in
this workstream. The full original-cache restoration gate remains for the lead
after real acceptance and compiled catalogue activation.
