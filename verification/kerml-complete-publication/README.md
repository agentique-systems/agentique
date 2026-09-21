# KerML complete publication overlay

Working milestone on `foundation/kerml-complete-publication-overlay`, from
`main` at `2e259ff63f94ccf685800cb54835c5cab5363561`. Operational v8 remains
unchanged. Publication acceptance follows ADR 0022, independently of complete
conformance coverage. Evidence follows ADR 0021. Original authority/library
bytes and generation-1 release obligations remain unchanged.

The kernel accepts deterministic derived AssociationOccurrences, normal
association validation and navigation, explanation DAGs and additive producer
stages. KerML uses that primitive for CrossSubsetting; no link side channel or
synthetic crossedFeature slot exists. The sealed complete-overlay builder runs
all subjects to closure and performs capability checks. Its existence alone is
not accepted publication.

## Relationship production audit

| Family | Canonical representation | Reason |
| --- | --- | --- |
| Library bases, inheritance and effective membership | Query facts | Identity of inherited declarations is retained; no copied Features or unnecessary relationship instances |
| CrossSubsetting | Derived relationship Element, owned chain and AssociationOccurrences for crossedFeature and crossingFeature | Required relationship identity, ownership, specific/general upcasts and inverse metamodel navigation |
| Positional parameter/end/result redefinition | Derived Redefinition Elements with canonical endpoint slots | Observable relationship identities and typing consequences |
| Cross and variable-domain TypeFeaturing / FeatureTyping | Derived relationship Elements with canonical endpoint slots | Required ownership and downstream typing/domain navigation; registry chooses storage |
| FeatureChaining and contextual results | Derived Features and owned FeatureChaining Elements | Ordered structure, source/result identity and ownership |
| Valuation/result Subsetting | Derived Subsetting Elements | Required specialization of contextual result identity |
| BindingConnector and its ends | Derived Elements, memberships and ReferenceSubsetting relationships | Binding role, ownership, end identity and ordinary endpoint navigation |
| Association related/source/target Type; Connector related/source/target Feature | Query projections | Ordered navigation over existing canonical end and typing/reference facts; no second fact store |

All Element-backed producers route properties through registry storage
capabilities: canonical slots where appropriate, association occurrences where
required. Derived occurrences carry profile-qualified RuleIds and depend on
their derived relationship and endpoint facts. Kernel queries expand these
explanations and their conservative graph-search evidence.

## Reproduction

Commands, exit codes, source/working-tree digests and output hashes are recorded
in `summary.json`. Raw logs and corpus JSON live only under ignored
`verification/generated/kerml-complete-publication/`.

Run `python verification/kerml-complete-publication/scripts/verify.py all` for
the required workspace, metamodel, Rustdoc and frontend checks. Focused kernel
and language tests must pass before the release-mode `publication_focus
--staged` example. Only after that focused gate succeeds, run the release-mode
`canonical_publication` example with fresh `--output=` and
`--conformance-output=` paths under generated storage. Conformance output does
not control the publication exit code.

On Windows, `scripts/observe.ps1` samples the exact workspace release executable
into ignored JSONL. `scripts/report.py --resources=...` retains only aggregate
resource observations and their raw artifact hash. Bounded producer batches,
shared overlay state, interned immutable explanations, immediate canonical
producer dependencies and deterministic additive merges carry the resource
contract; there is no timing threshold. The required Rust checks use two build
jobs and disable debug symbols/incremental compilation to avoid exhausting the
workspace drive with reproducible debug artifacts.

`capability-status.json` and `publication-result.json` record the actual gate
outcome. They must not be inferred from successful compilation or from a partial
overlay. Accepted facade, manifest regeneration and authored consumption remain
conditional on the full publication gate.

The first full release attempt was stopped during stage 1 after private memory
reached approximately 11.60 GB (sampled maximum). Stage 0 had built 27,293 derived Elements and
302 occurrences. It did not establish publication acceptance. Inspection found
that query evidence treated original declared collection dependencies as reads
of later derived extensions. The repair preserves both origins separately and
shares immutable explanations; focused preservation tests precede any resumed
final expansion. Raw observations and the stopped command's actual nonzero exit
code remain in generated evidence and the command summary.

After the provenance repair, the focused release gate passed: 1,338 derived
records reached closure, all 2,312 capability subjects had no findings, all ten
retained references matched canonical endpoints, both Triggers namespaces were
Complete, and all 31 roles validated. The same run executed a separate
`KerMlConformanceReport` with Incomplete coverage. This focused result does not
promote the full publication capability matrix.

The second full attempt was stopped before closure at stage 1, 11,776 of
56,433 subjects. Sampled maximum private memory was 5.24 GB. Producer plans
still copied proofs before kernel interning, including already published facts;
DAG validation also copied every edge into forward and reverse indexes.
Sharing now starts in producer plans and kernel enqueueing. Existing canonical
proofs are reused after the same class/rule/value checks, and iterative Tarjan
validation borrows adjacency with auxiliary storage proportional to vertices.
The synthetic `agq-kernel --example shared_derivation_scale` gate passed with
10,240 facts, 16,777,216 derived dependency edges and two shared proof sets.
Its measured executable exited 0 at approximately 8.1 MB private memory.
Exhaustive four-vertex cycle tests and producer batch/reuse tests also passed.
These resource checks do not substitute for focused or full semantic acceptance.

The subsequent focused release gate also passed. All semantic result fields
matched the preceding passing run exactly (the separately executed conformance
report was excluded from that comparison). Closure again contained 1,338 derived
records, with zero findings across 2,312 capability subjects and 31 validated
roles. This is the focused prerequisite for the final full expansion.
