# ADR 0004: Borrowed KerML typed views

Status: implemented for the existing normative Root/Core closure.

`agq-kerml` is the KerML-specific Rust interface over `agq-kernel`. It registers
the pinned KerML 1.0 descriptors and provides readable constants and checked,
borrowed views. A view contains exactly an `ElementId` and `&ModelView`. There are
no copied property fields, secondary object graph, parser, runtime state, or
language inheritance encoded through Rust inheritance.

## Boundaries and authority

| Layer | Responsibility |
| --- | --- |
| Generic `agq-kernel` | IDs, immutable canonical records and snapshots, descriptor registration, structural subtype/redefinition checks, validated slots, reference indexes and evidence-bearing overlays. No KerML names or dependency on `agq-kerml`. |
| `agq-kerml` | Normative descriptor registration, generated identity constants, borrowed typed views, checked casts and property projection. Depends only on `agq-kernel`. |
| Future KerML semantics | Rule evaluation, relationship-derived properties, semantic specialization, namespace/member resolution, defaults and constraint evaluation. Must supply revision-bound results and evidence; typed views do not run these rules. |

The generated scope remains the 29 Root/Core classes in ADR 0003, including
their abstract supertypes and the existing structural closure. No Kernel-layer
classes or SysML descriptors are added. All 196 property IDs have constants;
the 50 association-owned ends remain metadata, not typed class accessors.
The 146 class property declarations supply accessors on their effective classes.
The supplied documents, library bytes, normative inputs, descriptor IDs and
descriptor golden remain unchanged.

## Public API

```rust
use agq_kerml::{classes, properties, views::{Feature, FeatureTyping}};

// `model` is borrowed from an immutable Snapshot or DerivedOverlay.
let feature = Feature::try_new(feature_id, model)?;
let name: Option<&str> = feature.declared_name()?;
let as_type = feature.as_type()?;
assert_eq!(as_type.id(), feature.id());
assert!(std::ptr::eq(feature.record(), model.element(feature_id).unwrap()));

let typing = FeatureTyping::try_new(typing_id, model)?;
let target_type_id = typing.r#type()?;
assert_eq!(typing.as_specialization()?.general()?, target_type_id);
assert_eq!(Feature::CLASS, classes::FEATURE);
// Writes use the kernel ChangeSet and normative property identities:
// changes.set(feature_id, properties::ELEMENT_DECLARED_NAME, value, origin);
```

The runnable crate-level rustdoc example constructs a complete Feature, including
the required primitive slots. The [integration fixture](../../crates/kerml/tests/typed_views.rs)
constructs Type, Feature, Membership, Specialization, FeatureTyping, Subsetting,
Redefinition and FeatureMembership records without parsing or modified descriptors.
It is a structurally validated model over normative descriptors, not a claim that
all KerML constraints have been evaluated.

`try_new` uses `ModelView::registry().is_subtype` and fails for absent elements,
unknown classes and invalid downcasts. Generated `as_*` methods and
`cast::<OtherView>()` are checked, preserve the model borrow and ID, and work with
additional registered subtypes. The sealed `TypedView` trait supplies common
capabilities. There are no public unchecked constructors or mutable view fields.
Views deliberately have no structural equality over model contents: compare
`id()` for semantic identity, and retain the model/revision context when comparing
state. Normative `Element.elementId` is model data, separate from kernel identity.

Accessor names are snake_case, with `r#type()` for the Rust keyword. References
return `ElementId`; callers can request a checked target view without allocation.
Strings are borrowed `&str`, Booleans are copied, and enums return their literal
identity. An optional scalar is `Result<Option<T>, ViewError>`; a required scalar
is `Result<T, ViewError>`. Many-valued properties return borrowed `Values<T>`, with
an outer `Option` when the lower bound is zero.

| Read result | Meaning |
| --- | --- |
| `Ok(None)` | Absent optional authored slot. |
| `Ok(Some(values))` with `values.is_empty()` | Present, computed/stored empty collection. |
| `NotComputed` | Derived property has no assertion in this model view, regardless of its lower bound. |
| `UnsupportedAssociationStorage` | This association shape still requires a canonical link store. |
| Other typed error | Missing required value, inapplicable property, incompatible domain or malformed slot. |

`Values::raw()` borrows the actual generic slot. `shape()` distinguishes scalar,
ordered, set and bag; `descriptor()` exposes the effective normative multiplicity,
ordering and uniqueness contract. Iteration preserves semantic order for ordered
values, duplicate occurrences for nonunique values, and deterministic storage
order for unordered values. Unordered iteration does not invent semantic order.
Reads validate every value's projected domain and the effective slot shape and
bounds before exposing an infallible iterator. This is O(n), without allocating
a replacement collection. The kernel already validates uniqueness and targets.

Properties resolve by `PropertyId` against the record's actual metaclass, never by
display name. For example, a Relationship view over FeatureTyping resolves
`Relationship.source` through `Specialization.specific` to `FeatureTyping.typedFeature`.
The many-valued supertype accessor iterates that single scalar and reports
`SlotShape::Scalar` and its effective descriptor. It does not create another slot
or array. Redefinition chains and narrowed reference types receive the same checks.

## Minimal prerequisite: associations with a single storage end

ADR 0003 conservatively refused every non-derived association write. That also
prevented constructing Specialization and FeatureTyping, whose endpoints are
required, non-derived association properties. This milestone narrows that refusal
without implementing a general link store or two writable association ends.

The generic registry now exposes `supports_slot_storage(PropertyId)`. A
non-derived association end can use a canonical slot only when:

1. It is class-owned and unique.
2. Its opposite is association-owned and non-navigable.
3. The opposite has multiplicity 0..* and is unordered.

The stored slot then carries each occurrence once. The existing incoming/outgoing
indexes are reconstructible views of those occurrences. There is no inverse
writable property, inverse bound or independently ordered inverse collection to
maintain. This choice follows metadata, not class names, `owned` prefixes or ID
ordering. The rule applies equally to declared and implied element construction.

This permits the normative relationship endpoint fixtures with their original
descriptors and normal ChangeSet validation. It does not evaluate subsets,
derived inverses or unions. An ancestor endpoint replaced by redefinition remains
illegal as a second slot. Both ownership pairs from ADR 0003 remain refused,
as do navigable, ordered or bounded inverse ends and nonunique authored ends.
Supporting those shapes still requires the previously selected canonical link
contract. No persistence format or repository API changes are introduced.

## Generated versus handwritten

| File | Ownership |
| --- | --- |
| `crates/kerml/src/generated/root_core.rs` | Existing generated normative descriptors and source-ID inventories. Unchanged. |
| `crates/kerml/src/generated/typed_views.rs` | Generated `metamodel`, `classes`, `properties`, and `views` modules; effective accessors, upcasts and exhaustive constant-identity checks. |
| `tools/metamodel-gen/src/typed_views.rs` | Handwritten emitter using the existing authoritative import, closure and validated descriptors. Rejects unsupported domains and Rust name collisions. |
| `crates/kerml/src/view.rs` | Handwritten checked-view macro, sealed capability trait, errors and allocation-free scalar/collection projection. No language-specific property tables. |
| `crates/kerml/src/lib.rs` | Registration facade, public exports and executable usage example. |

Run from the repository root:

```sh
cargo run --locked --offline -p agq-metamodel-gen
cargo run --locked --offline -p agq-metamodel-gen -- --check
```

The typed file carries all input hashes, the descriptor golden hash and generator
version `agq-kerml-typed-views/1`. The generator emits/checks IR, descriptors, golden
and typed views together. Generation has no build-time or runtime dependency in
`agq-kerml`. Naming changes affect convenience paths; they do not allocate new IDs.
The existing byte-currentness tests and CLI stale-output tests cover the new file.

## Verification and next semantic-query slice

[Recorded verification](../../verification/kerml-typed-views/README.md) includes the
required workspace and frontend/browser checks. Tests cover first-class relation
identity, pointer sharing, immutable snapshots, runtime subtype checks, ordered
and unordered access, repeated reference occurrences, missing computations,
malformed values, scalar narrowing through upcasts and descriptor-ID agreement.
Overlay assertions in tests are labelled fixture evidence, not normative rule
implementations. Passing structural checks does not establish semantic conformance.

Next, add direct relationship queries over the existing indexes: typings of a
feature, immediate specializations of a type, and direct subsettings/redefinitions.
Return the relationship IDs and provenance with endpoints, scoped to one model
view. Keep transitive semantic inheritance and namespace lookup separate. Scoped
ownership queries still need canonical storage for the two ownership pairs;
normative derived collections need an explicit ordering/completeness contract
and evidence before they can be reported as computed.
