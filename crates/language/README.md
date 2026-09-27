# agq-language

LanguageCore: a deliberate subset of SysML v2 held in memory as an element
tree. SysML text is the storage format: it is parsed into the tree on load and
printed from it on save. No dependencies, no UI, network, async or AI types.

What is supported, partial and excluded is in [docs/subset.md](../../docs/subset.md);
departures from the standard are in [docs/deviations.md](../../docs/deviations.md).

## API

```rust
use agq_language::{parse, print, validate, Source};

let tree = parse(&[Source::new("UrlShortener.sysml", text)]); // text -> Tree
let diagnostics = validate(&tree);                              // Vec<Diagnostic>
let sources = print(&tree);                                     // Tree -> canonical text
```

- `Tree` holds documents and their elements. `tree[id]` reads an element;
  `add`, `remove`, `get_mut` edit it. `find("P::A::x")` and `qualified_name(id)`
  help tools and tests.
- `Element` is one generic shape: a `kind` (`PartDef`, `Part`, `Port`,
  `Interface`, `Satisfy`, `Import`, `Doc`, ...), `name`, `owner`, `children`,
  `location`, and the properties the subset needs (`typed_by`, `specializes`,
  `redefines`, `multiplicity`, `direction`, `is_end`, `value`, `ends`,
  `target`, `by`, `text`). Fields a kind does not use stay empty.
- `ElementId` is opaque and stable across edits; parsing assigns ids in
  document order. Names are never identity.
- `Diagnostic` names the element (`ElementId`), its source location when
  known, a short stable `code` and a plain-language `message`.

`parse` never fails: text outside the subset becomes an `Unsupported` element,
text that cannot be parsed a `SyntaxError` element. Both keep their source
verbatim, print back unchanged and are reported by `validate`, so every
problem stays visible after later edits. (`parse` therefore returns only the
tree; `validate` is the one place that reports.)

## Check a model

```text
cargo run -p agq-language --example check -- models/url-shortener
```

Prints each problem as `file:line:column: error[code] Qualified::Name: message`,
then timings for parse, validate, print, and a single edit (renaming the first
part) with re-validation. Exit code 1 when there are problems.

## Design

- `lexer.rs`, `parser.rs`: hand-written recursive descent for the subset.
  A failed member is recovered at its own `;` or `}`; the enclosing body goes on.
- `printer.rs`: canonical formatting; `doc` comments survive, `//` notes do not.
- `resolve.rs`: name lookup. Owned members, then inherited members (through
  types, specialisations, subsetted and redefined features: lookup, never
  copying), then imports. Several candidates are an ambiguity error.
- `validate.rs`: the validity rules; each run re-resolves the whole tree,
  memoising only within the run.
- `library.rs`: the built-in `ScalarValues`, parsed once and shared. Its ids
  start at 2^48 so a resolved reference is always just an `ElementId`.

## Parser decision and measurements

The retained `agq-kerml-syntax` chart parser was measured on the URL
shortener (3.3 KB): 55–165 ms per parse in a debug build and 24–32 ms in
release, producing 1,456 production nodes that would still need lowering into
this tree. Its published grammar profile also rejects forms the specification's
own examples use (`end port`, `satisfy R by x`). The hand-written parser
parses the same file in 0.4 ms (debug) and 0.15 ms (release) and reports
constructs outside the subset by name.

The check command on Windows 10 (single runs):

| | debug | release |
|---|---|---|
| URL shortener, 115 lines: parse | 0.39 ms | 0.16 ms |
| validate | 1.4 ms | 0.23 ms |
| rename a part + re-validate | 1.1 ms | 0.20 ms |
| 20 renamed copies, 2,300 lines: parse | 6.3 ms | 1.8 ms |
| validate | 22 ms | 3.2 ms |
| rename a part + re-validate | 22 ms | 3.5 ms |
