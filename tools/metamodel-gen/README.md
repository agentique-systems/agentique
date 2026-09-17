# Metamodel importer foundation

Offline tooling for the pinned KerML 1.0 abstract syntax. The workspace package is
`agq-metamodel-gen`, binary `metamodel-gen`. It produces neutral JSON IR, not a
runtime descriptor registry or an executable language implementation.

```sh
cargo run --locked --offline -p agq-metamodel-gen
cargo run --locked --offline -p agq-metamodel-gen -- --check
cargo test --locked --offline -p agq-metamodel-gen
```

The default root is this crate's repository; `--root PATH` selects another checkout.
`--output FILE` selects another generated output. `--check` compares exact bytes
and never writes. Exit status is nonzero for bad input, hash drift, cross-check
failure, missing/stale output or unsupported arguments. All source hashes are
verified before parsing. There is no network access or `build.rs`.

`profile` defines the accepted namespace-aware XMI serialization subset; `xmi`
extracts/validates structural IR; `ir` defines rich records and descriptor keys;
`cross_check` independently compares overlapping JSON serialization facts;
`pipeline` verifies the local lock and assembles the deterministic bundle.
Fixtures exercise failure behavior; integration tests import the actual pinned
artifacts and compare the committed output, including across CLI invocations.

[ADR 0002](../../docs/adr/0002-normative-metamodel-pipeline.md) specifies preserved
information, defaults, intentionally opaque rules, exact cross-check limits,
dependency rationale, identity encoding and deferred runtime decisions.
