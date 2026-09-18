# Multi-metamodel importer and descriptor generator

Offline tooling for pinned KerML 1.0 and SysML 2.0 abstract syntax. The workspace package is
`agq-metamodel-gen`, binary `metamodel-gen`. It produces neutral JSON IR plus a
dependency-closed normative Root/Core Rust descriptor slice and a source golden
manifest. It also imports SysML with source-qualified KerML dependencies and the
same JSON cross-check pipeline. SysML runtime emission is a subsequent gate.
It does not execute language rules. [ADR 0007](../../docs/adr/0007-multiple-metamodel-baselines.md)
records the explicit baseline profiles, identity policy and representation differences.

The default command also generates/checks the SysML structural readiness audit.
That report currently says **blocked**; import currentness is not runtime support.
`--baseline sysml-2.0 --require-runtime --check` returns nonzero on this blocker.
See [exact evidence and resumption requirements](../../docs/sysml-v2-runtime-blocker.md).

```sh
cargo run --locked --offline -p agq-metamodel-gen
cargo run --locked --offline -p agq-metamodel-gen -- --check
cargo test --locked --offline -p agq-metamodel-gen
```

The default root is this crate's repository; `--root PATH` selects another checkout.
`--output FILE` alone selects IR-only export mode. `--descriptor-output DIRECTORY`
redirects Rust/golden paths and can accompany `--output`. With neither option,
all four KerML artifacts plus the SysML IR are generated or checked.
`--baseline kerml-1.0` or `--baseline sysml-2.0` selects one profile;
`--output` without a baseline preserves the KerML IR-only interface.
`--check` compares exact bytes
and never writes. Exit status is nonzero for bad input, hash drift, cross-check
failure, missing/stale output or unsupported arguments. All source hashes are
verified before parsing. There is no network access or `build.rs`.

`profile` defines the accepted namespace-aware XMI serialization subset; `xmi`
extracts/validates structural IR; `ir` defines rich records and descriptor keys;
`cross_check` independently compares overlapping JSON serialization facts;
`pipeline` verifies the local lock and assembles the deterministic bundle.
`descriptors` computes structural closure, validates generic descriptors and emits
`crates/kerml/src/generated/root_core.rs` plus
`standards/generated/kerml-1.0/root-core.golden.json`. `typed_views` emits
`crates/kerml/src/generated/typed_views.rs` from that same closure: metamodel/class/
property constants, borrowed views, checked upcasts and effective property readers.
Do not hand-edit generated files. No additional classes are selected for views.
Fixtures exercise failure behavior; integration tests import the actual pinned
artifacts and compare the committed output, including across CLI invocations.

[ADR 0002](../../docs/adr/0002-normative-metamodel-pipeline.md) specifies preserved
information, defaults, intentionally opaque rules, exact cross-check limits,
dependency rationale, identity encoding and deferred runtime decisions.
[ADR 0003](../../docs/adr/0003-normative-root-core-descriptors.md) supplies concrete
property evidence, the exact slice inventory, canonical association policy and
remaining semantic obligations. `agq-kerml` depends only on `agq-kernel` at runtime;
its dependency here is test-only for comparison of compiled descriptors to XMI.
[ADR 0004](../../docs/adr/0004-kerml-typed-views.md) describes the typed API, generated
versus handwritten boundaries and bounded association-slot support.

## Complete structural diagnostics

`cargo run --locked --offline -p agq-metamodel-gen -- --audit-full` emits the
complete raw `full.golden.json` and diagnostic `full-audit.json` for both baselines.
Use `--baseline` to select one language; SysML includes the complete KerML input.
`--check` is read-only and verifies report currentness, including blocked reports.
The normal generator also checks/emits these reports alongside existing artifacts.
The historical PartDefinition/PartUsage closure report remains separate.

`--require-runtime` requires complete translation and atomic registration for
the selected baseline. It no longer treats successful bounded KerML generation
as full runtime readiness. Both complete baselines currently fail; no complete
KerML expansion or SysML runtime has been generated. All raw facts remain in
the manifests. See [foundation review](../../docs/language-core-foundation-review.md)
and [ADR 0009](../../docs/adr/0009-property-redefinition-context.md).
