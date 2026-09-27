# SysML subset (R-8)

What `crates/language` understands. References are linked to the element
they name when loaded and keep that identity across renames and moves; the
printer writes a name that resolves back to it. Anything else is parsed as an
**unsupported** element: its text is kept verbatim (printed back unchanged),
it is reported with code `unsupported`, and it is never validated. A reference
to such an element is reported as unsupported too, never guessed.

Departures from the standard are listed in [deviations.md](deviations.md).

## Supported

| Construct | Notes |
|---|---|
| `package` | Nested packages; `public` / `protected` / `private` members |
| `import X::*;` `import X::Y;` | With `public` / `private`; public imports are re-exported; a top-level import serves its own document |
| `doc /* ... */`, bare `/* ... */` comments | Survive load and save; `//` notes are dropped |
| `part def` / `part` | |
| `port def` / `port` | Conjugated typing `~P`; directed features `in` / `out` / `inout` |
| `item def` / `item` | |
| `attribute def` / `attribute` | |
| `connection def` / `connection` | `end part` / `end port` / `end item` ends; `connect a.b to c.d`; bare `connect a to b;` |
| `interface def` / `interface` | Port ends (`end port p : P;` or `end p : P;`); `connect a.b to c.d` |
| `requirement def` / `requirement` | `subject s : T;`, `doc` as the requirement text |
| `satisfy R by x;` | `assert satisfy` is accepted; the `assert` is implied |
| Specialisation `:>` / `specializes` | Definitions of a compatible kind |
| Typing `:` / `defined by` / `typed by` | One or more types |
| Subsetting `:>` / `subsets`, redefinition `:>>` / `redefines` | `:>> x` without a name takes the name `x` |
| Implied redefinition | An `end` redefines the end at the same position of each general; a `subject` redefines the general requirement's subject (derived, never written) |
| Usages without a kind keyword | `x : T;`, `:>> x = 5;` and `ref x : T;` are reference usages; printed without a keyword |
| Multiplicity `[n]` `[n..m]` `[n..*]` `[*]` | Integer bounds |
| `abstract` | Definitions and usages |
| Quoted names `'Order Line'` | |

## Partially supported

| Construct | What works | Reason for the limit |
|---|---|---|
| Feature values | `= literal` (integer, real, string, `true`/`false`) | Expressions need an evaluator; Stage 4 |
| Value checking | Literals against the built-in scalar types | User value types need expression typing |
| Connections | Binary; plain feature chains as ends | Scenario A needs no n-ary or named ends |
| Feature chains | Connection ends and `satisfy ... by`; steps after the first are simple names | Enough for `a.b.c`; typing by chains is rare |
| Requirements | Subject, doc text, attributes, satisfy | Constraints need expressions |
| Standard library | `ScalarValues` only, built in | No runtime bundle (R-4); grows by need |

## Deliberately excluded (for now)

| Construct | Reason |
|---|---|
| `action`, `state`, `calc`, `constraint`, `use case`, `analysis`, `verification` | Behaviour; added with simulation (Stage 4) |
| `flow`, `message`, `succession`, `bind`, `allocation`, `perform`, `exhibit` | Added when simulation or allocation needs them |
| `require` / `assume` constraints, `actor`, `stakeholder`, `frame`, `concern` | Need constraint expressions |
| `enum def` / `enum` | Not needed by Scenario A yet; cheap to add |
| `occurrence`, `individual`, `snapshot`, `timeslice`, `event` | Time and individuals are not modelled |
| `ref part` and other `ref` + kind usages, `::>` reference subsetting, `=>` crossing, `derived`, `constant`, `variation` / `variant` | Not needed; each adds semantics to check |
| Initial / default values `:=`, `default` | Need expressions |
| `ordered`, `nonunique`, multiplicity expressions | Integer bounds are enough so far |
| Short names `<id>`, `alias` | One name per element keeps identity and display simple |
| `metadata`, `#` and `@` annotations, the `comment` keyword (named or `about`), `rep` | `doc` and bare comments cover documentation |
| `import all`, recursive `::**` imports, import filters | Plain imports are enough |
| `view`, `viewpoint`, `rendering` | Presentation is not part of the model |
| `library package`, `standard library package` | The library is built in |

## Validity rules

Diagnostic codes reported by `validate`:

| Code | Rule |
|---|---|
| `syntax` | Text could not be parsed (kept verbatim) |
| `unsupported` | A construct outside the subset, or a reference to one |
| `unresolved` | A name cannot be found from where it is written, or a linked chain step is no longer a feature of the step before |
| `removed-target` | A linked reference points at an element that was removed |
| `ambiguous` | A name matches several different elements |
| `duplicate-name` | Two members share a name, or an owned feature hides an inherited one without redefining it |
| `wrong-type` | A usage's type is not the right kind of definition (a part by a part def, a port by a port def, ...); `~` on a non-port |
| `wrong-kind` | Specialisation, subsetting, redefinition, satisfy or connection end names the wrong kind of element |
| `specialization-cycle` | A definition specialises itself, or a feature subsets itself |
| `composition-cycle` | A part def contains itself through parts with a lower bound of at least 1 |
| `redefines-unknown` | `:>> x` does not name a feature inherited by the owner |
| `incompatible-ends` | Connected features do not fit the definition's ends, or two ports do not fit each other (see deviation 8) |
| `misplaced-end`, `misplaced-subject`, `duplicate-subject` | `end` outside connection/interface defs; subject outside a requirement; more than one subject |
| `bad-multiplicity` | Lower bound above upper bound |
| `wrong-value` | A literal does not fit its built-in scalar type |
| `wrong-subject` | `satisfy ... by x`: `x` is not of the requirement subject's type |
