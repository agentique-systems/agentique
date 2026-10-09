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
| `ref part` / `ref item` *(C-55)* | A referential usage: it refers to a part (or item) that exists elsewhere instead of containing one (SysML 7.6.3). `ref part supply : PowerBus;` declares the role; `= bus` (or `ref part :>> supply = bus;` in a usage) binds it to a part usage, and no copy is made. Without a value it refers to nothing in that configuration. A part without `ref` is composite: it is contained, and exists only with its owner. A connection end through a referential part faces the other end (what it refers to is not inside). Model execution runs a bound referential part as the instance it refers to (slots, ports and state machine are that instance's, and traces name it); an unbound one has only its ports, so connections through it hold, and a message reaching it stops the run (`missing-stand-in`) unless a scenario stand-in answers for it |
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
| Usages without a kind keyword | `x : T;`, `:>> x = 5;` and `ref x : T;` are reference usages; printed without a keyword. A reference usage is referential by definition: `:>> supply = bus;`, giving an inherited part a value, binds it as `ref part :>> supply = bus;` does |
| Multiplicity `[n]` `[n..m]` `[n..*]` `[*]` | Integer bounds |
| `abstract` | Definitions and usages |
| Quoted names `'Order Line'` | |
| `enum def` / `enum` *(C-50)* | Values are written `enum a;` (or `a;` inside the enum def) and used as `E::a`; an attribute may be typed by an enum def |
| `dependency from A to B;` *(C-50)* | One client, one supplier; optionally named and with a body (`doc`) |
| Expressions *(C-50)* | Literals, `null`, names and feature chains (`job.code`, `Decision::allow`), unary `-` and `not`, `* / %`, `+ -`, `< <= > >=`, `== !=`, `and`, `xor`, `or`, `implies`, `if c ? a else b`, `new T(a = x, b = y)` (named arguments), parentheses. Names inside are references like any other: linked by identity, printed by a name that leads back |
| `exhibit state s { ... }` *(C-50)* | The behaviour of a part def or part: `entry;` (or `entry` with an action) then `then S;` for the first state, `state S;` with `entry` / `exit` actions, and transitions |
| `transition [name first] S accept ... if g do e then T;` *(C-50)* | Trigger `accept x : T via port` (or `accept T via port`) or `accept after d`; a guard; one effect (`send`, `assign` or `action { ... }`) |
| Action nodes *(C-50)* | `send e via port`, `assign x := e`, `if c { ... } else { ... }` (and `else if`), `accept x : T via port`, `accept after d`, `action [name] { ... }`; members of an action run in the order written, printed with `then` after the first |
| `verification def` *(C-50)* | A scenario: `subject`, `objective { verify r; }`, stand-ins (usages of `Scenarios::StandIn`), steps (action nodes) and checks (`assert constraint name { e }`) in order |

## Partially supported

| Construct | What works | Reason for the limit |
|---|---|---|
| Feature values | `= literal` (integer, real, string, `true`/`false`) or `= expression` (C-50) | Units, sequences and function calls are not supported |
| Value checking | Literals against the built-in scalar types; enum values against the declared enum def | Expressions are checked for their names, not typed statically; a runner reports a type mismatch where it meets one |
| Behaviour | Flat state machines, one per part, are what runs (ROADMAP §4.14); nested states are read and validated | Runners report nested states, `do` actions of states and several behaviours of one part as unsupported |
| Durations | `accept after d` with `d` in milliseconds of logical time | No units library (deviation 14) |
| Connections | Binary; plain feature chains as ends; `end` features need a kind keyword (`end part a`) outside interface defs | Scenario A needs no n-ary or named ends |
| Feature chains | Connection ends and `satisfy ... by`; steps after the first are simple names; typing, subsetting or redefining by a chain is unsupported | Enough for `a.b.c` |
| Requirements | Subject, doc text, attributes, satisfy | Constraints need expressions |
| Standard library | `ScalarValues`, built in; plus Agentique's own `Agents` and `Scenarios` (deviations 12 and 13) | No runtime bundle (R-4); grows by need |

## Deliberately excluded (for now)

| Construct | Reason |
|---|---|
| `action def`, `state def`, `calc`, `constraint def` and `constraint` usages outside checks, `use case`, `analysis`, `verification` usages, `perform`, `exhibit` of a state defined elsewhere, `parallel` states | Not needed by Scenario I; each adds semantics to run and check |
| `accept at`, `accept when`, `send ... to`, `terminate`, `merge`, `decide`, `fork`, `join`, `while`, `for`, `loop` | Not needed by Scenario I |
| `flow`, `message`, named `succession`, `bind`, `allocation` | Added when a scenario needs them |
| `require` / `assume` constraints, `actor`, `stakeholder`, `frame`, `concern` | Need constraint expressions |
| `occurrence`, `individual`, `snapshot`, `timeslice`, `event` | Time and individuals are not modelled |
| `ref` with kinds other than `part` and `item` (`ref port`, `ref attribute`, `ref action`, ...), `::>` reference subsetting, `=>` crossing, `derived`, `constant`, `variation` / `variant` | Not needed; each adds semantics to check (attributes and directed features are referential anyway) |
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
| `unreachable-target` | No name written at the reference leads back to its target (the target is private, hidden by another element with that name, or inside an unnamed element), so saving would lose the link |
| `missing-target` | A `satisfy` or `import` without the name it needs |
| `wrong-end-count` | A connection or interface usage with other than two ends (or none) |
| `ambiguous` | A name matches several different elements |
| `duplicate-name` | Two members share a name, or an owned feature hides an inherited one without redefining it |
| `wrong-type` | A usage's type is not the right kind of definition (a part by a part def, a port by a port def, ...); `~` on a non-port |
| `wrong-kind` | Specialisation, subsetting, redefinition, satisfy or connection end names the wrong kind of element |
| `specialization-cycle` | A definition specialises itself, or a feature subsets itself |
| `composition-cycle` | A part def contains itself through composite parts with a lower bound of at least 1; a referential part contains nothing, so a part def may refer to its own kind |
| `redefines-unknown` | `:>> x` does not name a feature inherited by the owner |
| `incompatible-ends` | Connected features do not fit the definition's ends, or two ports do not fit each other (see deviation 8) |
| `misplaced-end`, `misplaced-subject`, `duplicate-subject` | `end` outside connection/interface defs; subject outside a requirement; more than one subject |
| `bad-multiplicity` | Lower bound above upper bound |
| `wrong-value` | A literal does not fit its built-in scalar type |
| `wrong-subject` | `satisfy ... by x`: `x` is not of the requirement subject's type |
| `wrong-value` | Also: a value that is not one of its enum def's values, or a literal for an enum-typed feature; a value of an enum def with a type or value |
| `wrong-value` | Also (C-55): a referential part or item whose value is not a feature chain naming a part usage (for `ref item`: a part or item usage) of every type the referential usage has; a composite part or item bound to another part or item usage (deviation 19) |
| `misplaced-behaviour` | A state, transition, action node or check outside the place it belongs (an exhibit state in a part; states and transitions in a state machine; `entry`/`exit` in a state; checks in a scenario) |
| `initial-state` | A state machine with states and no `entry; then S;`, or with several |
| `incompatible-send` | `send` of an item through a port with no `out` item of that type (seen from a scenario, which stands outside its subject: no `in` item) |
| `incompatible-trigger` | `accept` of an item through a port with no `in` item of that type (from a scenario: no `out` item) |
| `missing-port` | `send` or a step's `accept` without `via port` |
| `wrong-fallback` | An agent's `fallback` is typed by a part def that does not specialise every contract the agent specialises (every general except `Agents::Agent`) |
| `agent-fallback` | An agent's `fallback` is itself an agent |
