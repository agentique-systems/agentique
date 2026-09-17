# ADR 0007: Source-qualified multi-metamodel import profiles

Status: implemented for offline KerML 1.0 and SysML 2.0 import/cross-check.
Runtime SysML generation is a subsequent gate. Refines ADR 0002's primitive-only
dependency boundary without changing descriptor-key v1.

`baseline.rs` explicitly declares specification, version, metamodel namespace,
exact serialized root URI, input lock, primary XMI/hash, JSON schema, dependencies,
output paths and descriptor target. Profiles are selected by ID, not inferred
from filenames. The CLI defaults to both languages; `--baseline` selects one.
PrimitiveTypes remains in the original KerML lock. No build script, network client
or runtime importer dependency is introduced.

The shared XMI reader resolves exact artifact URI plus XMI fragment into pinned
dependency keys, checking classes, properties, packages, enums/literals and retained
operation references by kind. Unknown references fail even inside retained bodies.
Operation-parameter stream flags are retained and Boolean-validated, not executed.
An ephemeral tooling graph qualifies keys and edges without changing entity keys
or their owning metamodel. Same display names and local IDs in different metamodels
do not collide. No KerML class acquires a SysML descriptor identity.

The pinned SysML XMI root URI spells `https://ww.omg.org/spec/SysML/20250201`.
The publication/JSON use `https://www.omg.org/spec/SysML/20250201`. The profile
records both and requires the exact input hash. The original attribute survives;
no heuristic URL repair or external-reference alias is introduced.

SysML JSON projects dependency classes into its own schema namespace. The existing
checker compares the combined source-qualified graph using that serialization
namespace; schema names never allocate descriptor IDs. Joins require unique names
and verified schema URIs; ambiguity fails. Both representation differences are
enumerated in the SysML bundle. API Schema.json is never an input.

The shared cross-check covers 175 classes, seven enums, 428 directly owned properties
and 209 direct generalizations. The SysML XMI itself contributes 93 classes.
Scalar nullability differences are enumerated without rewriting XMI multiplicities.
Abstractness, ordering, opposites, derived unions, defaults and rules remain outside
the JSON comparison. Imported class counts are not runtime language coverage.

Tests cover deterministic/current imports, unchanged KerML output, external direct
and transitive inheritance, preserved keys, kind failures, unresolved metaclasses,
same-name isolation, wrong versions/artifacts and stale read-only CLI checks.
All run offline. See [gate evidence](../../verification/sysml-language-v2/README.md).
Runtime descriptors/views, semantic rules, text and library ingestion remain later
gates; execution and application migration remain excluded.
