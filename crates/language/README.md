# agq-language

LanguageCore: a deliberate subset of SysML v2 held in memory as an element
tree. SysML text is the storage format: it is parsed into the tree on load and
printed from it on save. No dependencies, no UI, network, async or AI types.

What is supported, partial and excluded is in [docs/subset.md](../../docs/subset.md);
departures from the standard are in [docs/deviations.md](../../docs/deviations.md).

## API

```rust
use agq_language::{parse, print, validate, link, Source};

let mut tree = parse(&[Source::new("UrlShortener.sysml", text)]); // text -> linked Tree
let diagnostics = validate(&tree);                                  // Vec<Diagnostic>
link(&mut tree);                                                    // after edits: link new names
let sources = print(&tree);                                         // Tree -> canonical text
```

- `Tree` holds documents and their elements. `tree[id]` / `get` read an
  element and `get_mut` edits its properties; its place in the tree changes
  only through `add`, `insert`, `move_to` and `remove`. `rekey` swaps parsed
  ids for stored ones. `find("P::A::x")` and `qualified_name(id)` help tools.
- `Element` is one generic shape: a `kind` (`PartDef`, `Part`, `Port`,
  `Interface`, `Reference`, `Satisfy`, `Import`, `Doc`, `Comment`, ...), `name`,
  `owner()`, `children()`, `location`, and the properties the subset needs
  (`typed_by`, `specializes`, `redefines`, `multiplicity`, `direction`,
  `is_end`, `conjugated`, `value`, `ends`, `target`, `by`, `text`). Fields a
  kind does not use stay empty.
- `Reference` is the one reference type: the name as written, one `Step` per
  feature-chain step, each with the `ElementId` it is linked to. A linked
  reference keeps its target when the target is renamed or moved; the printer
  then writes a name that resolves back to it. `Tree::references_to(id)` lists
  the references pointing at an element; `Reference::target()` gives the
  linked target.
- `ElementId` is opaque; `from_raw` / `raw` convert for storage. Removed ids
  are never handed out again. Ids from 2^48 up belong to the built-in library.
- `Diagnostic` names the element (`ElementId`), its source location when
  known, a short stable `code` and a plain-language `message`.

`parse` never fails: text outside the subset becomes an `Unsupported` element,
text that cannot be parsed a `SyntaxError` element. Both keep their source
verbatim, print back unchanged and are reported by `validate`, so every
problem stays visible after later edits. (`parse` therefore returns only the
tree; `validate` is the one place that reports.) Unlinked references are
resolved by name on every `validate`, so an edit that creates a missing target
fixes them; `link` then records the target.

## Check a model

```text
cargo run -p agq-language --example check -- models/url-shortener
```

Prints each problem as `file:line:column: error[code] Qualified::Name: message`,
then timings for parse (with linking), validate, print, and a single edit
(renaming the first part) with re-validation and printing. Exit code 1 when
there are problems.

## Design

- `tree.rs`: elements, references, ids and the structural operations.
- `lexer.rs`, `parser.rs`: hand-written recursive descent for the subset.
  A failed member is recovered at its own `;` or `}` outside nested braces;
  the enclosing body goes on.
- `printer.rs`: canonical formatting; comments survive, `//` notes do not.
- `resolve.rs`: linking and name lookup. Owned members, then inherited members
  (through types, specialisations, subsetted and redefined features: lookup,
  never copying), then imports; then the document's top level and the
  library. Several candidates are an ambiguity error. Implied redefinitions
  (positional ends, subjects) are derived here and never written.
- `validate.rs`: the validity rules, following linked targets; each run
  memoises only within the run.
- `library.rs`: the built-in `ScalarValues`, parsed once and shared. Its ids
  start at 2^48 so a resolved reference is always just an `ElementId`.

## Parser decision and measurements

The retained `agq-kerml-syntax` chart parser was measured on the URL
shortener (3.3 KB): 55–165 ms per parse in a debug build and 24–32 ms in
release, producing 1,456 production nodes that would still need lowering into
this tree. Its published grammar profile also rejects forms the specification's
own examples use (`end port`, `satisfy R by x`). The hand-written parser
parses the same file in 0.4 ms (debug) and 0.15 ms (release) before linking, and reports
constructs outside the subset by name.

The check command on Windows 10 (single runs; parse includes linking):

| | debug | release |
|---|---|---|
| URL shortener, 115 lines: parse | 1.1 ms | 0.30 ms |
| validate | 1.4 ms | 0.32 ms |
| print | 0.95 ms | 0.26 ms |
| rename a part + re-validate + print | 2.7 ms | 0.73 ms |
| 20 renamed copies, 2,300 lines: parse | 20 ms | 3.9 ms |
| validate | 26 ms | 3.8 ms |
| rename a part + re-validate + print | 42 ms | 6.0 ms |

Validation includes checking that every linked reference still prints as a
name that leads back to its target. Twenty packages that all import each
other validate in a few milliseconds (a test guards this).
