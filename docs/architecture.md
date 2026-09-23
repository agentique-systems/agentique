# Agentique architecture: two generations

Agentique is a Rust-native compiler, semantic database and canonical model engine
that will underpin language services, modeling and execution. The canonical
semantic graph is its center:

```text
KerML / SysML source -> lossless syntax -> linking / resolution
    -> semantic elaboration -> canonical kernel graph
    -> queries / validation / APIs / views -> execution IR -> simulation
```

KerML canonical publication is a prerequisite, not Agentique's end goal. It is
distinct from KerML conformance completion. Once an immutable canonical KerML
publication passes its acceptance contract, the next phase is the pinned SysML
Systems Library textual foundation and `agq-sysml-semantics`, composing KerML
queries over the same kernel graph. Definition and Usage semantics come before
Part, Item, Attribute, Port and Connection; modeling and execution follow that
language foundation. Incomplete validator coverage remains explicit and does not
justify extending the KerML phase after canonical publication succeeds.

The publication convergence review separates strict kernel validity, canonical
KerML publication and conformance coverage ([ADR 0022](adr/0022-kerml-publication-vs-conformance.md)).
Incomplete validator coverage alone does not block a canonical semantic graph.
Operational v8 explicitly resolves the owned-cross featuring-domain and imported
Membership collision interpretations. Its independent authority manifests extend
v7 without rewriting earlier profiles. Operational v9 has passed whole-corpus
producer closure, all publication capabilities and all 4,000 mandatory references,
with no blocking authority conflicts. Dependency-driven producer frontiers and
bounded reference reuse remain the publication architecture; scoped success alone
cannot seal a canonical publication. A symbolic
multiplicity-bound reference demonstrated a publication-blocking consequence
of the retained v8 domain decision. Operational v9 adds exactly the authorized
KERML11-4 owning-Namespace and KERML11-3 cross-multiplicity interpretations.
Published through v8 retain their semantics. Accepted canonical publication
permits advancing the operational default to v9. All 31 accepted bindings and
actual two-project authored integration have passed. The substrate is complete;
SysML textual and semantic foundation work has begun. See
[ADR 0023](adr/0023-operational-multiplicity-context-v9.md) and the
[v9 evidence](../verification/summaries/kerml-v9-publication/README.md).
See the historical
[convergence evidence](../verification/summaries/overnight-convergence/README.md)
and the earlier [publication evidence](../verification/kerml-canonical-publication/README.md).
New evidence follows [ADR 0021](adr/0021-verification-evidence-policy.md).

The operational errata publication v3 milestone adds explicit published and
Agentique operational KerML 1.0 profiles ([ADR 0014](adr/0014-operational-standard-errata-profiles.md)).
Authored generation-2 KerML and standard-library construction explicitly use the
operational profile; legacy descriptor/registry APIs retain published metadata.
The reviewed KERML11-81 deletion is independently verified. That historical
milestone stopped at Gate 7 on the separate
[KERML11-140 resolution authority conflict](kerml-operational-library-publication-authority-conflict.md).
That historical milestone did not accept a library facade; the v9 result above
supersedes its publication status without rewriting its authority findings.

The initial [structural foundation review](language-core-foundation-review.md)
exposed Integer/Real domain, association-inheritance and Property-context gaps
([ADR 0009](adr/0009-property-redefinition-context.md)). Its earlier blockers are
historical: the structural runtime v3 boundary described below now includes
`agq-kerml` and `agq-sysml` descriptors and borrowed views. That structural
registration does not establish SysML textual or semantic support.

Generation 1 is the functioning integrated application described below. Generation 2
is the standards-driven engine for all new language implementation; it does not yet
power the application. Compatibility/migration work must name its generation.
Keep the [v0.1 release coverage](../standards/coverage.json) and requirements intact;
use [generation-2 status](../standards/v2-coverage.json) for new engine capabilities.

The shared importer and runtime descriptors cover the pinned SysML 2.0 graph.
The [published-property blocker](sysml-v2-runtime-blocker.md) records an earlier
stage; the current SysML textual and semantic foundation is described below.

[ADR 0008](adr/0008-property-subsetting-cycle-semantics.md) corrects the former
combined property-cycle invariant: cyclic subsetting is representable and exact
source anomalies are separately reported. That earlier runtime review exposed an
independent invalid redefinition context in the required SysML closure; the
subsequent structural runtime milestone superseded that blocker.

The historical modeling-platform milestone's prerequisite check reproduced this
blocker on its fetched `main`; its stages 0-6 have not begun. See the
[platform prerequisite record](../verification/modeling-platform-v2/README.md)
for the exact base, commands and resumption conditions. Repository, API, view and
transformation boundaries must be designed against the completed generation-2
language stack before they are shown as implemented components here.

## Generation 2: language engine under development

Arrows point from a consumer to its dependency. The generator is maintenance
tooling and emits checked-in descriptors; it is never a runtime or build-script
dependency.

```mermaid
flowchart BT
  kerml[agq-kerml] --> kernel[agq-kernel]
  semantics[agq-kerml-semantics] --> kerml
  text[agq-kerml-text] --> syntax[agq-kerml-syntax]
  text --> semantics
  text --> kerml
  syntax --> kernel
  sysml[agq-sysml: descriptors and views] --> kerml
  sysmlsem[agq-sysml-semantics] --> sysml
  sysmlsem --> semantics
  text --> sysml
```

Canonical data lives in generic immutable kernel records. Borrowed typed views
project those records without copied fields or a second graph. Language semantics
reuse evidence-bearing queries; inherited elements retain their original identity.
Parsers preserve text and supply declarations/references, never canonical truth or
name denotation. Source identity and semantic identity remain distinct. Unsupported
syntax, unresolved references and unimplemented semantic rules remain explicit.

The SysML dialect extends the existing lossless production arena in
`agq-kerml-syntax`; `agq-kerml-text` uses reusable Definition/Usage family contracts
and the existing kernel construction machinery. All 21 pinned Systems Library
documents parse under explicit Operational v2 with zero recovery and unchanged
source bytes. The separately selectable Published grammar retains 13 complete
parses and its eight exact failures. All 21 documents support declared canonical
construction, producing 7,591 elements and 1,327 source reference assertions;
those counts do not establish endpoint or semantic completeness.

`agq-sysml-semantics` has no parser dependency. Its 13-field dependency contract
binds the accepted KerML Operational v9 publication, profiles, descriptor graphs,
library identities, grammar and semantic manifests, bindings and rule sets into
the shared query-context identity. An unpublished construction overlay allows
structural implications and reference refinement to cooperate. The dependency-
driven scheduler supports combined KerML/SysML closure on new Systems records;
the accepted KerML publication remains shared and sealed, without producer replay.
Definition/Usage typing, specialization, ownership, inherited identity sets,
subsetting, redefinition and names reuse KerML queries. Attribute/Item/Part
projections and structural Port/Connector projections retain their evidence.
Current-graph answers do not certify unfinished SysML producer closure.

No Systems publication is accepted. Mandatory reference and combined producer
closure remain pending. Operational v2 records the three bounded canonical target
corrections while Published and Operational v1 remain reproducible.
The immediate acceptance path is the monolithic dependency-driven scheduler and
its existing Structural/ContextualBindings strata. Compositional publication is
proposed research in [ADR 0027](adr/0027-compositional-semantic-publication.md),
not current publication authority or an additional foundation gate. Its semantic subject
dependency planner preserves potential writers and negative/provider searches;
document boundaries do not establish independently sealable components. Planning
and component evidence audits remain distinct from issuing a sealed stratum.
The existing global closure guards stay in force until narrower footprints have
an equivalent proof. Current read-side footprints cannot yet prove useful strata;
this does not establish semantic indivisibility of Systems. See the
[compositional publication evidence](../verification/summaries/compositional-publication/README.md).
`CanonicalSysmlSystemsLibrary` rejects any failed construction,
reference, producer, capability, binding, provenance or authority gate. Canonical
identities retain their source provenance through lowering and refinement;
authored edits do not change the immutable library dependency. See the current
[language stability bridge evidence](../verification/summaries/language-stability-bridge/README.md)
and the historical
[Phase 1 foundation evidence](../verification/summaries/sysml-semantic-foundation/README.md).

Normative targets are KerML 1.0 and SysML 2.0. Metamodel import, runtime descriptor
closure, semantic queries, textual grammar and library ingestion have distinct
coverage. Generation 2 has no application migration, persistence repository,
Systems Modeling API or execution. See the foundation details below and
[semantic kernel guide](semantic-kernel.md), including ADRs 0001–0006.

## Generation 1: integrated v0.1 application

The Engine is implemented here. It does not wrap a modelling application. Language
meaning comes from the three supplied OMG publications; product and experiment
scope comes from the supplied HTML. The original files and embedded engineering
artifacts remain unchanged. See [standards discrepancies](standards-discrepancies.md).

```mermaid
flowchart BT
  syntax --> model
  semantics --> syntax
  workspace --> semantics
  simulation --> semantics
  application --> workspace
  application --> simulation
  storage[SQLite adapter] --> application
  assistant[Assistant provider adapter] --> application
  server[HTTP / model service adapter] --> application
  server --> storage
  server --> assistant
  cli --> application
  console[React Console] --> server
```

`model`, `syntax`, `semantics`, `workspace`, and `simulation` have no database,
HTTP, browser, or provider dependency. Their public APIs accept values and return
values or diagnostics. `agentique simulate` uses those APIs without opening a
workspace database. `Application` centralizes typed proposals, authority,
revisions, scenarios, experiment controls, and durable acknowledgement. CLI,
HTTP, and Assistant do not implement independent model semantics.

## Language implementation decision

A handwritten lexer, recursive descent declaration parser, and Pratt expression
parser were selected for the bounded textual subset. The evaluation criteria were
UTF-8 byte spans, recovery without changing source, partial declarations, explicit
unsupported syntax, expressions, and control over stable identity reconciliation.
Parser generators such as Pest would automate recognition but would still require
the same lossless source store, recovery policy, abstract-syntax construction and
semantic resolver. Tree-sitter would provide incremental syntax but would require
a maintained SysML/KerML grammar and the same separate semantic implementation.
No complete, verified third-party Rust SysML semantic implementation was assumed.
The repository's executable evaluation is `agq-syntax`'s grammar/recovery,
malformed-source and exact-expression tests, followed by independently validated
SysML fixtures. This was a scoped engineering comparison, not a comparative
performance benchmark of three parser implementations.

The parser constructs declarations, scoped references, expressions, comments and
source ranges; it is not regular-expression extraction. Original source is the
lossless concrete representation. Unknown syntax remains in source and in
diagnostics. It is not rewritten from a reduced abstract syntax tree. Semantic
expansion attaches explicit rule-labelled relationships and derived elements.
See the five-axis [coverage register](../standards/coverage.json) for the precise
distinction between interpreted constructs, retained source and executable scope.

## Identity, edits and persistence

Authored declarations receive UUIDs. Qualified names and paths locate declarations;
they are not their identity. Reviewed rename and move edits update source references
by resolved token spans and reconcile IDs into the next revision. Derived elements
use a UUID derived from their owning declaration and semantic role. Runtime
occurrences use fresh IDs independent of either category. Explicit external-source
reconciliation accounts for every previous ID; stale or unused mapping entries are
rejected. Source replacement is intentionally stricter than guessing a rename.

An invalid draft has its own durable record and diagnostics. A candidate change set
cannot replace the accepted revision until validation succeeds. Proposals carry
their base revision and exact source diff; commits are atomic and stale bases fail.
Old runs and their manifests never acquire the current model's names or sources.

SQLite is the local deployment alternative to the specification's proposed
PostgreSQL infrastructure: a single local project does not require an external
database service. The application depends on a typed Store contract, not SQLite.
WAL, FULL synchronous commits, record checksums, an exclusive process lock and
atomic transactions implement acknowledgement. A transaction includes objects,
audit event, and idempotency receipt. In-memory head/run caches change only after
commit. Immutable manifests and appended trace records are stored separately from
compact checkpoints. Run candidates share immutable plans and trace snapshots;
trace appends use copy-on-write so a failed durable commit cannot change a prior
snapshot. A server restart marks unfinished runs interrupted; it does
not resume them. A CLI invocation may attach to a paused run, but cannot share the
same database with another Engine process. Schema versions newer than supported
are rejected rather than overwritten. Private database records are not interchange.

The actor comes from the authenticated adapter: a local operator session or the
single Assistant. JSON cannot supply an actor. This deliberately moves the proposed
contract's `actor_id` from untrusted input to the authenticated command envelope.
Receipts are keyed by actor and command ID, and include the canonical payload hash.
The canonical form is serde serialization of typed values with ordered maps; this
is Agentique's private digest contract, not a claim of RFC 8785 canonicalization.

## Experiment semantics

AGQ-SEQ-01 prepares one resolved flat exclusive state occurrence. It pins model,
scenario, libraries, contract, Engine build, ordered inputs, parameter values,
transitions, source references and limits. Initialisation follows the succession
immediately after empty entry. A step checks the stop condition, next ordered input,
matching receiver and payload type, and all eligible guards; exactly one transition
must be enabled. No match and multiple matches block without consuming input.
Unsupported effects, time, hierarchy, parallelism, explicit executable inheritance,
non-unit executable multiplicity and unsupported expressions block preparation.
Structural connections and other behaviours are expressly excluded from the plan.

Integer comparisons use arbitrary-precision integers (4096-digit limit), never
floating point. Scalar type identity is resolved against the pinned ScalarValues
declarations. Scenario values cannot contradict an authored fixed literal. Guards
are pure; type witnesses used in preparation never become runtime defaults.
There are no effect providers or ambient I/O capabilities in the runtime.

Semantic traces exclude wall-clock timestamps. Normalization excludes generated
run/occurrence IDs while retaining model identity and logical order. A completed
experiment is distinct from a check verdict; documentary requirements remain
`not_run`. Input exhaustion, ambiguity, limits, interruption and evaluation errors
are separate outcomes. A failed step changes diagnostics/status only, not the
committed semantic state or input cursor.

The declared memory budget is deterministic runtime accounting: eight times the
serialized immutable plan plus 16 KiB, and 2048 bytes per trace record. It is a
conservative reservation for the bounded value structures, not an operating-system
RSS limit. Actual process memory is measured separately by `tools/benchmark.ps1`.
The application retains full traces in memory and loads stored revisions on
demand; aggregate workspace memory is not subject to a global quota. Source work
is bounded to 2,000 files, 32 MiB total, 8 MiB per file, 200,000 tokens per file,
100,000 declarations, nesting 128, and 100 edits per change set. These are explicit
allocation/work limits rather than a claim to cap process RSS. `WorkControl`
publishes lexing, parsing, resolution, validation and expansion progress and
cooperatively cancels before commit. A mutex serializes cancellation against the
start of durable commit; cancellation arriving after that boundary is refused.
The server allows at most eight pending command jobs and sixteen background
operations, retaining 64 job results. CLI/in-process callers use the same work
contract without requiring an HTTP executor. Lost job responses are recovered
using the original idempotent command ID and durable receipt.

## Console and Assistant

The React Surface projects real Engine declarations, references, behaviour edges,
source text and run history. The graph is a view of resolved transition endpoints;
the tree, tables and properties provide non-diagram inspection. Layout is local
presentation state. Scenario JSON is experiment configuration and never defines
states or transitions. Source, rename, type, value, move and connection changes use
the same candidate/commit boundary. Source and modelling jobs expose progress and
cancellation; run history pages through all stored records in batches of 1,000.

The Conversation carries project, selection, revision and optional run context.
The deterministic adapter exercises real typed tools. The live adapter uses an
environment-configured chat-completions transport and permits exactly one typed
tool selection per request. Its responses are grounded in actual Engine results;
it does not fabricate a model description or a successful mutation. Both use the
same approval workflow. Approvals bind the actual Assistant actor, reviewed payload,
base revision and expiry, and are consumed once. The provider has no approval tool.

The server binds loopback, requires a local session bearer credential for APIs, and
rejects cross-origin API requests. This is a single-user local application, not a
multi-tenant deployment. `AGENTIQUE_ASSISTANT=test` is visibly labelled. Provider
failure leaves manual Engine operations available.

## Interchange and API

Text export emits the original `.sysml`/`.kerml` source and a clearly private
identity sidecar. `.kpar` is a ZIP project with standard top-level `.project.json`
and `.meta.json`, language URI, project dependencies, index and SHA256 checksums,
plus the private identity extension. It is not a renamed database archive.
Unsupported extra archive entries are refused to avoid lossy export. No full
interchange-conformance claim is made.

The separate `/api/model` adapter maps a read subset of Systems Modeling API 1.0:
projects, main branch, commits, elements and roots, with standard paging parameter
names and Link headers. The element result is a partial metaclass projection, not
a complete OMG abstract-syntax serialization. The API coverage register enumerates
every published operation and its status. All simulation routes are exclusively
under `/api/agentique`.

## Standards-driven foundation

The additive generic `agq-kernel` foundation is described in ADR 0001. The intended
runtime dependency direction is `agq-kernel` <- `agq-kerml` <- `agq-sysml`.
Language-specific descriptors and rules belong above the kernel.

[`tools/metamodel-gen`](../tools/metamodel-gen/) imports hash-pinned KerML 1.0 MOF
XMI into a neutral IR and cross-checks overlapping facts in the normative JSON
serialization schema. It is engineering tooling, not canonical runtime storage;
no runtime crate reads its neutral JSON IR. Generated Rust descriptors are compiled
into `agq-kerml`. [ADR 0002](adr/0002-normative-metamodel-pipeline.md)
records authority, identity, retained property semantics and the decisions needed
before generating runtime descriptors. Existing language coverage is unchanged.

`agq-kerml` now depends only on `agq-kernel` and registers a generated, structurally
closed Root/Core descriptor slice. [ADR 0003](adr/0003-normative-root-core-descriptors.md)
records normative property evidence, inheritance resolution and canonical association
boundaries. Runtime crates do not depend on the importer. Existing application
language support and execution coverage remain unchanged; descriptor registration
does not implement KerML rules.

`agq-kerml` also supplies generated constants and borrowed typed views for this
same slice. [ADR 0004](adr/0004-kerml-typed-views.md) describes the API and its
minimal generic association-slot prerequisite. A typed view stores an element ID
and a reference to the existing immutable kernel model; it is not another
canonical object. The kernel remains language-neutral.

`agq-kerml-semantics` evaluates pure ownership, membership, specialization, direct
typing/subsetting/redefinition and bounded effective-feature queries above these
views. [ADR 0005](adr/0005-kerml-semantic-query-foundation.md) defines full semantic
context identity, positive/search dependencies, completeness and proof graphs.
A minimal generic ordered-many/scalar-inverse association storage shape enables
declared ownership without precomputed language facts. Inherited features retain
their kernel identities. No parser, cache framework or execution behavior is added.
New kernel consumers use the query facade; the older application model remains
separate pending an explicit migration.

The additive second-generation textual frontend is `agq-kerml-syntax` (immutable
source revisions, lossless tokens/range CST, borrowed syntax views) and
`agq-kerml-text` (source history, conservative edit reconciliation and lowering).
[ADR 0006](adr/0006-lossless-kerml-text-frontend.md) records the parser comparison,
normative slice, recovery and identity contract. Text lowers directly through
`agq-kerml` descriptors to `agq-kernel` snapshots; neither crate uses `agq-model`.
Name denotation remains in `agq-kerml-semantics`, whose context includes pending
specialization scope obligations during working-model resolution. SourceOrigin
pins document, source revision, UTF-8 byte range and optional syntax identity;
canonical records hold no parser objects. The kernel can transactionally update
element provenance while preserving historical snapshots and semantic identities.

The slice accepts named namespaces, types with required specialization, features,
feature typing, subsetting and redefinition. Relationships and owning memberships
are first-class records. Unsupported or malformed headers do not assert semantics;
valid incomplete bodies can still form a working model. Unresolved/ambiguous
references remain explicit assertions outside the structurally valid snapshot.
`validate_slice` distinguishes this working state from successful bounded syntax,
resolution and semantic-query checks. It is not full KerML validation or executable
verification. This historical bounded slice is extended by the canonical KerML
publication and shared SysML frontend described above. Application,
HTTP and simulation continue to use their existing implementation.


## Structural runtime foundation v3 (supersedes bounded registry status)

The complete KerML 1.0 descriptor graph is now the production `agq-kerml`
registration boundary. `agq-sysml` adds all 93 SysML 2.0 classes using the same
KerML identities; both depend inward on the generic kernel. This historical
structural milestone did not include a SysML semantic rule layer, parser,
library ingestion or application migration. The current foundation adds the
textual and query layers described above; application migration remains separate.

[ADR 0011](adr/0011-structural-registration-vs-metamodel-conformance.md) separates
atomic structural registration from metamodel-authoring conformance. Exact source
anomalies remain visible and strict validation rejects them. Raw property relations
are distinct from class slot replacement and association inheritance. The complete
[runtime contract](language-core-runtime-contract.md) describes exact numeric
carriers, association occurrences, five derived states and semantic context identity.
Earlier blocker findings remain historical evidence; the appended foundation review
and v3 verification record provide the current gate result.
