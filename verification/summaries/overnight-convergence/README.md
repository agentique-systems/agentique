# Overnight convergence

Base: fetched `origin/main` at `ed2cf3a9cc086e165c075c591a3892472250e9b7`, clean
worktree. Branch: `foundation/agentique-overnight-convergence`. Source work used
isolated kernel, semantic closure and hygiene/authored-fixture worktrees; the lead
integrates commits and owns whole-corpus acceptance.

## Implementation

- Immutable producer frontiers use metaclass applicability and indexed positive
  and bounded negative query dependencies. Existing proof/record identities and
  ordered ownership remain canonical; inherited elements are never copied.
- The reference full scan retains its own full-graph stability comparison and
  original batched planner. Fixtures compare records, occurrences, explanations,
  computation searches, queries, capabilities and digests across four traversal
  orders and three batch sizes.
- Operational v8 closure first stabilizes structural implications, then emits
  reference-result and FeatureValue bindings whose nearest context can otherwise
  change. Valuation subsettings remain in the structural phase. New binding/end
  subjects still enter normal producer closure, and dirty prior readers are
  checked again; obsolete scalar targets are never silently retained.
- Additive overlay handoff transfers unshared accumulated maps. Persistent proof
  interning and fact-to-proof-to-premise cycle validation avoid repeated large
  dependency copies. Structural validation/index rebuilding remains per frontier.
- Negative-search evidence is also interned and shared across producer outputs
  and immutable overlays. Exact unions preserve all search keys. Validated empty
  worklist frontiers reuse their input; actual changes still rebuild indexes.
- Private producer/status subqueries carry shared searches until their proof or
  read-set boundary. Public query evidence stays eager. Atomic replacement of
  scheduler read populations now uses dense sorted arrays instead of a second
  tree per subject.
- Explicit status-only query outcomes avoid producing an unused explanation
  forest. The ordinary evidence-bearing query API remains available.
- A sealed synthetic library with all 31 roles exercises two authored projects,
  imports, aliases, typing, specialization, subsetting, redefinition, visibility,
  shadowing, profile rejection, shared records, protected facts and parallel readers.
- Independent real-corpus slices share declaration preparation when requested.
  Scoped results never issue `CompletePublicationOverlay`.
- Accepted binding serialization requires the canonical facade. Regeneration and
  stale checking reuse the same accepted in-memory publication.
- Immutable dependencies protect ownership against newly introduced local
  carriers, including composite slots, association occurrences and derived
  navigation. Direct writes and inverse operations already protected library
  facts; the final-candidate guard also rejects taking ownership indirectly.

## Evidence and workflow observations

[Command observations](commands.json) and [development checks](development.json)
record actual commands, exits and tested source identities; raw data is ignored.
[Repository cleanup](../repository-hygiene.md) removed 916 redundant files,
149,337,759 bytes, while retaining authority and offline proof inputs.

After incoming source-role indexing, the unchanged semantic scale workload passed:

| Declared subjects | Derived Elements | Derived occurrences | Test-body seconds | Peak private bytes |
| ---: | ---: | ---: | ---: | ---: |
| 1,920 | 768 | 256 | 0.22 | 44,556,288 |
| 15,360 | 6,144 | 2,048 | 1.73 | 278,503,424 |
| 60,000 | 24,000 | 8,000 | 7.01 | 1,052,094,464 |

All three use two producer rounds. The 60k case evaluated 28,000 subjects, skipped
68,002 consideration events by applicability, attempted 140,000 producer families,
interned 80,000 proof sets and reused 100,000; 744,000 dependency edges were
considered. There were three overlay materializations, including the empty input.
This is approximately proportional growth over the measured range, not a semantic
acceptance criterion. Compilation was separately monitored and is excluded from
the table's test-body timings. All 203 semantic/text package tests passed at the
preceding integration point; the index changes additionally passed the kernel and
focused source-role/reference-equivalence suites.

After the producer-phase fix, the same 60k gate passed again: 6.97 seconds in the
test body (7.172 seconds monitored wall time), 1,050,771,456 peak private bytes,
and identical deterministic work counters and derived fact counts. Separate
small changing-context fixtures cover both producer phases and their ordering
equivalence; this scale shape specifically exercises shared endpoint fan-out,
crossing occurrences and bounded negative searches.

The final shared-query version passed the unchanged 60k gate in **5.82 seconds**
in the test body (6.188 seconds monitored wall time), at **977,530,880 peak private
bytes**. It still produced 24,000 Elements and 8,000 occurrences, evaluated 28,000
subjects, attempted 140,000 families and considered 744,000 dependency edges.
The unchanged graph now needs two materializations and one validated empty-frontier
reuse. Its 664,000 logical search entries retain 256,000 entries in 8,000 shared
sets. Zero existing-fact reuse is counted in this shape because the second,
empty frontier no longer rebuilds and recounts earlier facts. The separate
254.282-second release compilation is excluded from the test-body measurement.

The indexed real `publication_focus` gate passed in 256.609 seconds, with
1,085,370,368 peak private bytes. From 29,140 declared records it produced 1,338
Elements and two AssociationOccurrences in three frontiers, evaluating 1,389
subjects and attempting 7,355 applicable producer families. The 2,312-subject
capability audit had zero findings; all ten retained mandatory references were
Complete with matching canonical endpoints, both namespace regressions passed,
and all 31 standard roles validated. This remains a scoped gate, not acceptance
of all 4,000 mandatory corpus references.

The additional ownership guard passed 15 focused kernel tests (11 derived
association tests and four immutable dependency tests). Workspace Clippy, the
three required metamodel stale/runtime checks, and all four browser tests passed
at the source identities recorded in the command summaries. Later source fixes
still require their appropriate integration checks.

The first workspace-test build was stopped during compilation after 148.172
seconds as Windows debug artifacts reduced free disk from about 13 GiB to 3 GiB.
It did not execute the suite. The subsequent command disables debug symbols and
incremental caches while retaining ordinary test optimization, debug assertions
and overflow checks. A new optional watchdog disk reserve stops before workspace
free space falls below 1 GiB; all nine watchdog regressions pass. Existing caches
remain untouched following the earlier automatic approval rejection.

The workspace rerun passed: **412 tests, zero failures, two explicitly ignored
scale gates**, across 105 result summaries, in 304.328 seconds including
compilation. Peak private memory was 1,025,343,488 bytes, and more than 3.2 GB of
disk remained free. The ignored semantic scale gate is run separately in release
mode; this workspace result does not substitute for that performance preflight.

After the final query-sharing and authority-report changes, the semantic package
passed 173 tests (two scale gates explicitly ignored), and text integration
passed all 46 tests in 137.157 seconds including compilation, with 1,009,401,856
peak private bytes. Kernel/semantic all-target Clippy and the subsequent text
all-target Clippy passed; the latter took 7.031 seconds. The updated coverage and
authority metadata also passed `npm run standards:check`. Earlier frontend and
metamodel gates are recorded at their actual tested source identities.

The old `early-sharing` corpus process was already running before this task.
The lead stopped it at 10,804.95 seconds elapsed, 3,522,764,800 private bytes and
2,360,279,040 working-set bytes. Its original wrapper subsequently appended the
actual terminated result to the historical complete-publication summary. This
is not a new worklist whole-corpus attempt.

Initial Python-launcher watchdog observations did not include native children.
Those metrics are explicitly superseded. The corrected Windows watchdog creates
the native command suspended, attaches it to a Job and resumes it only afterward.
Native descendant/no-survivor, memory, wall, stop-marker, progress and failed-monitor
tests pass. A direct kernel scale run measured 182 MiB peak private memory and
3.953 seconds; it covers 60k declared subjects, 30k derived Elements, 20k derived
occurrences, three stages and shared proofs. Timing is workflow evidence only.

Integration caught and fixed a formal-target context comparison that accidentally
compared the previous frontier's graph digest as an invariant. Profile, library
and resolved target contracts remain checked. A separate scoped-input regression
rejects missing requested subjects rather than silently claiming Complete.

Three diagnostic focused corpus commands were deliberately stopped during source
refinement as implementation fixes arrived. They are not semantic passes and do
not count as whole-corpus publication attempts. No accepted binding manifest or
operational default transition follows from those runs.

The third diagnostic stopped after 459.922 seconds, at 3,399,475,200 peak private
bytes, after six refinement rounds. Construction took 1.1–1.3 seconds per round
and context hashing approximately 0.12 seconds; resolution rose from 18.8 to 88.8
seconds. Those observations directed subsequent work to query premises, naming
memoization and compact invalidation storage. The first semantic scale gate also
hit its development memory stop; [focused workstream evidence](workstreams.md)
records that failure alongside the smaller kernel scale success.

## Acceptance

Publication is blocked by the concrete symbolic-bound reference witness below.
No new whole-corpus attempt has been launched. The operational default and
existing binding manifest remain unchanged. KerML conformance coverage
remains incomplete and separate. SysML semantic work has not started.

The later A-only result from the A/B fail-fast command (`1393a35`) converged in
419.969 seconds with 2,045,722,624 peak private bytes, exit 1 and no resource stop.
Declaration/reference preparation is included. Its 7,889 selected declarations
from 21 documents produced 8,442 Elements and 120 AssociationOccurrences over
six frontiers. It evaluated 13,849 subjects, retained 3,342,750 scheduler read
edges, and reused two empty frontiers (five materializations including the empty
input). The 10,916,786 logical search entries retained 3,885,584 entries in 3,346
shared sets. This run combines sharing and scope corrections; it is not an
isolated measurement of either change. B did not run after A failed.

A controlled rerun after private query sharing and dense scheduler read storage
used the same selection and source identities. It completed in **268.797 seconds**
at **1,835,253,760 peak private bytes**, exit 1 with no resource stop: 36.0% less
wall time and 10.3% less peak memory. Every deterministic work counter and the
final diagnostic set matched the preceding A run. The final report also includes
the candidate graph digest, an additional reporting step absent from that earlier
failure report. This remains a failed publication preflight, with capability and
mandatory-reference audits deferred; faster convergence cannot authorize the
missing binding. No full-corpus attempt or SysML phase follows from it.

The watchdog's first producer progress samples were at 203.422 and 203.438 seconds
respectively. The remaining command time fell from 216.547 to 65.359 seconds;
these sampled intervals include remaining closure work and report generation,
not a separately timed pure producer benchmark. Source identities, all counters
and the final diagnostics compared equal (comparison exit 0). The final candidate
digest is `48f2b4b169e47093bd4d523a42eef39951fde6f5849207f54bf6f2ac453cf6e1`.

Final acceptance gates deliberately remain open: A is incomplete, B stopped after
A's failure, C–E have not passed, and the full capability/reference audit has not
run. The retained `publication_focus` pass predates the last allocation changes;
the final semantic suite, text suite, scale gate and controlled A comparison cover
those changes. Required workspace, frontend and metamodel checks were run at the
integration checkpoints recorded above; no new public language crate was created.

The sole final producer diagnostic is `KQ_REFERENCE_CONTEXT` on
`3f630d5e-a2f0-5bdb-a20e-33d03f0970d3`, the `instantNum` reference at bytes
1813..1823 in pinned `Kernel Semantic Library/Transfers.kerml`:
`Transfers::Transfer::instant[instantNum]`. Both capability and mandatory-reference
audits were explicitly `not_run` after incomplete producer closure. Stage
observations are retained in ignored generated storage, and the exact command,
source identity and output hash are in [commands.json](commands.json).

The retained [multiplicity decision](../../kerml-publication-convergence/authority-blockers.json)
fixes an empty domain when `Multiplicity::owningType` is absent, and explicitly
rejects substituting the lexical owning Namespace. The bound-expression domain
must match that domain. The existing reference-binding correction requires a
direct or indirect expression featuring context for the referent, and explicitly
leaves a missing context Incomplete. Here the referent is featured by `Transfer`.
The [arbitrary-name structural fixture](../../../crates/kerml-semantics/tests/publication_v8_bounds.rs)
reproduces that shape independently of library identities: worklist and full scan
converge to the same graph, proofs, searches, query answers and digest while
retaining the final missing-context obligation. This fixture is paired with the
actual pinned-source observation above; neither implies an authority correction.

Consequently, [the current authority impact](../../kerml-canonical-publication/authority-decisions.json)
now classifies KERML11-4 as publication-blocking, retaining its previous
validation-only classification and evidence. The earlier assessment deferred
bound-expression accessibility and missed its required reference-binding
consequence. No replacement domain, profile successor or unrelated validator has
been implemented. Publication and conformance reporting tools read the same
current impact record instead of hard-coding the earlier classification.

The first indexed slice run exposed a real producer conflict in slice A's second
frontier: an existing derived TypeFeaturing was proposed with a different
`featuringType`. The declared dependency slice contained 9,880 subjects from 22
documents; its first frontier produced 8,141 Elements and 120 occurrences.
The lead stopped the shared A–E command during B at 333.641 seconds and
3,180,486,656 peak private bytes to investigate, rather than repeat the same
failure across the remaining slices. Exit 124 records that requested stop;
neither the failed slice nor the unfinished slices passed. No new whole-corpus
publication attempt has been launched.

A bounded A-only diagnostic reproduced the conflict in 273.375 seconds at
3,172,601,856 peak private bytes (exit 1). It identified the `seq` reference in
`SequenceFunctions::size`'s `size(tail(seq))`: invocation specialization changes
its selected reference-binding domain from the enclosing Function to the
InvocationExpression. The conflicting property is
`TypeFeaturing::featuringType`, not an authority/profile decision. Independent
small valuation and invocation fixtures exercise this phase dependency.

After the phase correction, slice A closed its structural frontier and no longer
reported the conflicting TypeFeaturing. Contextual binding frontiers added 4,383
and 974 Elements, then reached a stable but Incomplete producer result. The
watchdog stopped the command at its 4 GiB preflight limit: 461.344 seconds,
4,335,812,608 peak private bytes, exit 124. Slice B did not run, and this is not a
whole-corpus attempt. Stage diagnostics now stream to ignored storage, and failed
producer closure skips later audits, so a resource stop cannot erase completed
frontier diagnostics or trigger unnecessary acceptance work.
