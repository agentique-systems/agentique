# ADR 0002: Pinned metamodel sources and an offline abstract-syntax importer

Status: implemented for the KerML 1.0 metamodel foundation. This decision adds
tooling; it does not claim KerML semantic or interchange conformance.

Later refinements: ADRs [0003](0003-normative-root-core-descriptors.md) and
[0004](0004-kerml-typed-views.md) implement runtime descriptors and views;
[0005](0005-kerml-semantic-query-foundation.md) and
[0006](0006-lossless-kerml-text-frontend.md) add bounded queries and text.
The deferred decisions below describe this import milestone, not current status.

## Baseline and authority

The [KerML 1.0 publication](https://www.omg.org/spec/KerML/1.0) identifies these
normative machine-readable documents. The dated `20250201` resource namespace
belongs to the published 1.0 baseline, not preliminary 1.1 material.

| Input | OMG identifier | Role in this pipeline |
|---|---|---|
| [KerML.xmi](https://www.omg.org/spec/KerML/20250201/KerML.xmi) | ptc/25-04-04 | Primary authority for the MOF abstract syntax, serialized using UML XMI. Packages, classifiers, inheritance, properties, associations, flags, bounds, defaults and rule bodies come from this file. |
| [KerML.json](https://www.omg.org/spec/KerML/20250201/KerML.json) | ptc/25-04-21 | Normative JSON Schema for the JSON representation of abstract syntax. Independent cross-check of overlapping serialization facts; never used as a substitute for MOF. |
| [PrimitiveTypes.xmi](https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi) | ptc/18-01-02 | Primary dependency: the five UML primitive declarations referenced by KerML XMI. Listed by the [UML 2.5.1 publication](https://www.omg.org/spec/UML/2.5.1). No additional files are required to resolve the imported types. |

Original bytes and per-input specification/version, URI, filename, SHA-256, size,
representation, media type, role, retrieval time and available HTTP metadata are in
[`standards/normative/kerml-1.0/`](../../standards/normative/kerml-1.0/).
The lock distinguishes September 2025 adoption from the March 2026 KerML PDF
publication. HTTP Last-Modified was not supplied and is recorded as null, not
invented. `.gitattributes` prevents line-ending conversion of pinned inputs.

Existing PDFs, HTML, standards locks and original/corrective libraries are
unchanged. `KerML-Model-Interchange.json` describes project metadata; the Systems
Modeling API `Schema.json` and `OpenAPI.json` describe API payloads. None is a
metamodel input. UML.xmi itself is not fetched or recursively interpreted: this is
an explicit importer profile for the serialization constructs actually used,
not a bootstrap implementation of UML/MOF.

## Location and dependency boundary

`tools/metamodel-gen` is a workspace tooling package, `agq-metamodel-gen`, exposing
the `metamodel-gen` binary and a small testable library. `tools/` makes its build
time role explicit alongside existing engineering tools. It is not a runtime
crate, canonical graph, database, parser frontend or simulation subsystem.

The intended runtime direction remains `agq-kernel` <- `agq-kerml` <- future
`agq-sysml`. No KerML symbols, grammar or rules enter `agq-kernel`. This milestone
does not create placeholder runtime language crates. The tool depends inward on
the kernel only to return its existing typed `MetaclassId` and `PropertyId` values.
No runtime crate depends on this tool or loads the generated JSON.

The only new registry dependency is `roxmltree` 0.21.1 (MIT/Apache-2.0); its
`memchr` dependency was already locked. Namespace-aware, read-only XML parsing and
source ranges are useful here. DTDs are disabled; no entities or references are
fetched. Existing serde/serde_json serialize the IR, sha2 checks source bytes, and
uuid maps external keys. `tempfile` is test-only. There is no build script, network
client, async runtime, web framework, database or language parser dependency.

## Import and retained information

The pipeline verifies all three local input hashes, imports primitive declarations,
validates the KerML XML profile and global ID uniqueness, extracts an IR, validates
reference kinds/closure and inheritance acyclicity, then cross-checks the JSON.
Only after these steps can the CLI emit output. Unknown elements, namespaces,
types, attributes, child contexts, unsupported external types, invalid bounds,
duplicate singletons/IDs and unresolved references are explicit errors.
The profile accepts both XMI namespace revisions present in the inputs (20161101
and 20131001) and resolves qualified type names by namespace, not prefix spelling.

`Metamodel` contains source provenance and maps of packages, classifiers and
properties, keyed by document-local XMI IDs. Classifier kinds distinguish classes,
associations, enums and primitive types. Each entity has a source-qualified key,
name, qualified path, UTF-8 source byte range and the explicitly supplied
attributes. A local reference is an XMI ID within that model; an external type is
the full pinned primitive URI and fragment. These references are not simple-name
descriptor identities.

The structured property record preserves owner, type target, lower/upper bounds,
ordering, uniqueness, derivation, derived-union, read-only, ID and aggregation
flags, redefinitions, subsets, association and other member ends. Association-owned
ends remain association-owned; they are not manufactured class slots. Opposites
are a list of other member ends, with member order retained rather than assuming
every association must be binary. Navigable owned ends are retained separately.
Three published ends are unnamed: the empty name/path segment is preserved and
the authoritative XMI ID still distinguishes each end.

Effective structural defaults follow UML: class abstractness false; multiplicity
1..1 when value nodes are absent; present LiteralInteger/UnlimitedNatural nodes
with no value mean zero; -1 (also accepted as `*`) denotes unlimited upper bound;
ordering/derivation/derived-union/read-only/ID false, uniqueness true, aggregation
none. Explicit source attributes and retained value nodes distinguish omission
from an authored value. No primitive domain is narrowed to Rust i64/f64 here.

Recognized operations, constraints, comments, enum documentation, generalization
records, defaults, package imports and MOF tags are retained as namespace-qualified
`SourceNode` trees. Their identifiers, attributes, bodies, children and source
order survive; OCL/English bodies are never parsed or executed. Retaining an opaque
body is not accepting its executable semantics. The importer is deliberately not
a complete UML constraint validator.

The pinned KerML input yields 24 packages, 82 classes, 2 enums, 131 associations
and 313 properties (210 class-owned and 103 association-owned). There are 79
property redefinition references, 207 subsetting references and 3 derived-union
properties. All encountered structural forms have explicit import handlers.
The 84 operations and 340 expression bodies are retained, not implemented.
Unknown future structural extensions fail, including inside retained content.

## Cross-check contract and discrepancies

The JSON adapter checks the exact class/enum definition set (excluding its
`Identified` serialization helper), fully qualified schema IDs, enum literal
sets, 88 direct class inheritance edges, and all 210 directly owned class
properties. Property comparison covers presence, scalar/array shape, primitive
kind, enum target, reference target via `$comment` plus the `Identified` `$ref`,
and array lower/upper bounds. Single class-reference nullability is also compared.
This deliberately avoids calculating inherited/redefined property resolution.
Simple names are used only to join this particular schema's definitions after
checking uniqueness and qualified `$id`; ambiguity fails and requires a mapping.

JSON nullability is not automatically MOF multiplicity. In these exact inputs,
36 scalar data properties allow JSON null although absent XMI value nodes give
an effective UML lower bound of one (for example `Element.elementId` and
`Feature.isComposite`). The cross-check normalizes the optional null branch for
scalar primitive/enum type comparison and enumerates every such case under
`nullable_data_scalars_with_positive_xmi_lower`. XMI bounds are not rewritten and
no undocumented exception changes the IR. Required data properties with explicit
bounds, such as `Comment.body`, also have their non-null value type checked.
Resolving the import/export meaning of this difference belongs to a later milestone.

The schema cannot independently certify abstractness, ordering, uniqueness,
derivation, composition, opposites, redefinitions/subsetting, defaults or rules.
These omissions are listed in the generated report. Comparison success only means
the stated overlapping facts agree; it is not semantic verification.

## Descriptor identity v1

The external key contains specification, version, metamodel URI, artifact URI,
exact artifact SHA-256, package path as an array, authoritative XMI ID and entity
kind. `DescriptorKey::encoded` serializes a fixed compact JSON tuple, beginning
with `agentique-descriptor-key/1`, in that order. Arrays avoid delimiter ambiguity.
Package names and class/property simple names are never the sole identity.

UUID-v5 uses private Agentique namespace `a63be8e5-2305-4377-88be-764db0bd5da6`
and those UTF-8 bytes. A class key maps through `MetaclassId::from_u128`; a
property key through `PropertyId::from_u128`. Wrong-kind conversions fail. This
is Agentique's policy, not an OMG-mandated formula. A golden encoding/UUID test
protects the policy. Artifact or version changes deliberately produce new IDs;
cross-release reconciliation will require an explicit migration policy.

## Determinism, reproduction and verification

`standards/generated/kerml-1.0/metamodel.json` is a checked-in neutral IR bundle,
including primitive provenance and the cross-check report. Maps use sorted keys;
ordered source lists retain their source order. UTF-8 pretty JSON uses LF and a
final newline. Wall clocks, retrieval headers and machine paths are excluded.
The artifact hash and byte ranges intentionally mean differently serialized XMI
is a different source, even if a full MOF tool might consider it equivalent.

From the repository root, after ordinary locked dependency provisioning:

```sh
cargo run --locked --offline -p agq-metamodel-gen
cargo run --locked --offline -p agq-metamodel-gen -- --check
cargo test --locked --offline -p agq-metamodel-gen
npm run standards:check
```

`--check` re-imports and compares exact bytes without writing. `--root` chooses a
checkout; `--output` chooses the output JSON file. Inputs are hash-checked even
without `--check`. Both the main verification runner and the normative integration
test detect a stale committed IR. CI provisions Cargo dependencies explicitly and
runs verification with `CARGO_NET_OFFLINE=true`; standards retrieval is never
part of a build/test. Browser tests use only the local server.

`node tools/pin-kerml.mjs` is a separate, explicitly invoked acquisition tool.
It reproduces the existing reviewed lock: all remote hashes must match before
any missing input is created exclusively. Changed upstream or local bytes fail;
existing files and lock metadata are never overwritten. A future normative
baseline needs a separately reviewed directory/lock, not automatic replacement.

## Deferred decisions

Complete runtime descriptors remain deferred. The kernel needs explicit contracts
for redefinition, subsetting, derived unions, association ends/opposites and
inheritance conflict resolution. No UML aggregation attribute is present in the
pinned KerML XMI; the KerML feature named `isComposite` is model data, not that
MOF flag. Containment obligations must be reconciled with the specification and
retained constraints, not guessed from `owned...` names. Scalar/enum domains and
defaults, JSON nullability, cross-version identity migration, operation/rule
execution, generated wrappers, parsing and SysML layering are subsequent work.
