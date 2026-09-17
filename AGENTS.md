# Agentique

Implement language semantics in `model`, `syntax`, `semantics`, `workspace` and
`simulation`; HTTP, SQLite and provider transport depend inward through application
contracts. Read `docs/architecture.md` and `standards/coverage.json` before changing
language support. Preserve the supplied HTML/PDFs and original library bytes.

Never accept unsupported executable semantics silently, equate run completion with
verification success, or mutate a checkpoint before a durable commit. Changes to
requirements must retain their authority and verification obligations.

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `npm run check`, `npm run build`, `npm test`, and
`npm run test:e2e`. Record actual results in `verification/`.
