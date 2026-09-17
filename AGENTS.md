# Agentique

New language semantics MUST target generation 2: `agq-kernel`, `agq-kerml`,
`agq-kerml-semantics`, `agq-kerml-syntax`, `agq-kerml-text`, the normative
metamodel generator, and future SysML language crates. Read `docs/architecture.md`,
`docs/semantic-kernel.md`, and `standards/v2-coverage.json` before changing language
support. Use pinned KerML 1.0 / SysML 2.0 authority, not preliminary 1.1 / 2.1.

Generation 1 (`agq-model`, `agq-syntax`, `agq-semantics`, old workspace/simulation,
application/server/console) remains the operational product. Extend its language
engine only for explicitly scoped compatibility or migration work. Preserve its
release obligations in `standards/coverage.json` and `verification/traceability.json`;
generation-2 progress does not establish generation-1 release completion.

Canonical records belong in the language-agnostic kernel. Language crates depend
inward; SysML reuses KerML. Typed views borrow `{ ElementId, &ModelView }`; syntax
trees are not canonical models. Never copy inherited elements for lookup. Reuse
semantic query contracts, including completeness, evidence and search dependencies.
HTTP, SQLite and provider transport depend inward through application contracts.
Preserve the supplied HTML/PDFs and original library bytes. Pin distinct artifact,
specification and library content identities; acquisition is never a build step.

Never accept unsupported executable semantics silently, equate run completion with
verification success, or mutate a checkpoint before a durable commit. Changes to
requirements must retain their authority and verification obligations.

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `npm run check`, `npm run build`, `npm test`, and
`npm run test:e2e`, `npm run standards:check`, and
`cargo run --locked --offline -p agq-metamodel-gen -- --check`.
Run focused checks at stage gates and Rustdoc for new public language crates.
Record actual commands, outputs and exit codes in `verification/`.
