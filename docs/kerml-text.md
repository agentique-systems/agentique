# Second-generation KerML text frontend

`agq-kerml-syntax` owns source bytes, token/trivia ranges, recovery nodes, syntax
identities and borrowed declaration views. `agq-kerml-text` manages immutable source
history and builds `agq-kernel` snapshots using `agq-kerml` descriptors. Resolution
and its evidence live in `agq-kerml-semantics`. The old model is not a dependency.

```rust
use agq_kerml_text::{Document, syntax::{ByteRange, TextEdit}};

let mut document = Document::new("feature Base; type T :> Base;")?;
let old = document.current().clone();
let name = old.syntax().source().find(" T ").unwrap() + 1;
document.edit(TextEdit {
    range: ByteRange::new(name as u64, (name + 1) as u64)?,
    replacement: "Renamed".into(),
})?;
let current = document.current();
let validated = current.validate_slice();
let canonical = current.snapshot();
assert!(document.source(old.syntax().revision()).is_some());
```

Names, keywords and relationship forms follow the supplied KerML 1.0 grammar:

```kerml
namespace Example {
    feature Scalar;
    feature Base {
        feature value : $::Example::Scalar;
    }
    abstract type Derived :> Base {
        feature replacement typed by $::Example::Scalar
            redefines $::Example::Base::value;
        feature constrained : $::Example::Scalar
            subsets $::Example::Base::value;
    }
}
```

The untyped namespace-level Features above are also Types in the normative
metamodel. They provide closed test bases without pretending to load standard
libraries. `type T;` is recovered with a diagnostic because the printed
TypeDeclaration production requires specialization or conjugation. This slice
supports specialization, including comma-separated targets. Symbolic `:`, `:>` and
`:>>` and their `typed by`, `specializes`/`subsets`, and `redefines` counterparts
are accepted. References may use basic or quoted unrestricted long names.

`SyntaxStatus::Success` means this supported syntax parsed without diagnostics.
`Recovered` retains the complete source and usable syntax. A valid header with an
unfinished body can construct a `WorkingModel`; malformed or unsupported headers
produce no semantic declaration, including their nested declarations. Unknown
bodies and string literals are opaque during recovery, preventing declarations
inside them from escaping into the surrounding namespace.

`WorkingModel::references()` exposes every supported relationship assertion and
its resolution status. Only resolved, correctly typed references appear as
canonical relationships. An unresolved assertion is not a dangling kernel slot.
`validate_slice()` returns a borrowed `ValidatedModel` only after syntax,
resolution, duplicate-name and bounded effective-feature checks pass. It does not
establish all normative constraints, implied library relationships or execution
verification. `snapshot()` and ordinary `queries()` inspect the canonical facts
that could be constructed; inspect pending assertions as well for working input.
Reference query results are pinned to the final model revision and their remaining
pending semantic scopes. Their context and evidence can be inspected separately.

Every canonical element and stored slot has a SourceOrigin containing a document,
source revision, UTF-8 range and syntax ID. `Document::source(revision)` retains the
source after subsequent edits. Snapshots remain usable after dropping the frontend;
the caller must retain the source history separately for source inspection.

`Document::edit` applies one byte-range edit to the current revision. Exact name
token edits and unambiguous formatting retain independently allocated semantic
IDs. Memberships follow retained declaration identity; unchanged unique reference
clauses retain relationship identity. Moves, retargeting headers, ambiguous
duplicates and whole-source `replace` have no preservation claim. Deletion followed
by recreation gets a fresh identity. Reconciliation never consults an older
revision to resurrect a deleted element. See [ADR 0006](adr/0006-lossless-kerml-text-frontend.md)
for the full contract and architectural comparison.

Default parser limits are 8 MiB, 200,000 tokens and nesting 128. Byte/token exhaustion
returns a typed source error without publishing a new current model. Excessive
nesting is retained as recovery syntax with a diagnostic. Parsing is complete
reparsing today; source history is in memory and has no automatic eviction.

Before SysML: expand normative descriptors and constraint validation, implement
inherited/imported and multi-document name resolution, load pinned library models,
then build SysML syntax on the same source/lowering boundary. Packages, annotations,
aliases, short names, multiplicity, expressions and explicit visibility remain
diagnosed gaps in this milestone. HTTP and simulation migration are separate work.
