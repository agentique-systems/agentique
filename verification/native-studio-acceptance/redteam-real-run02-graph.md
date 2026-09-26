# Graph expert review of the real run02 gallery

Reviewed actual `03-graph-world.png`, `07-agent-view.png`, `06-history-diff.png` and `09-candidate-diff.png` under `verification/generated/native-studio-acceptance/real-run02/gallery`. These are the real self-model on native binary SHA-256 `fb2fedafc3d2e1574f526d1763cc99521cce64b4e7ad058e29665d616004e30e`, not the adversarial fixtures. This is one review/change iteration; updated native capture and regression execution are pending.

The six-element Repository Graph is traceable: selected incident paths are brighter, relationship labels explain ownership/typing/specialization, and the Inspector retains the authored selection and inherited port. The ten-result agent neighborhood clearly identifies its provider, intent, exact target revision and temporary read authority. Its 46% fit produces small/truncated names and substantial unused vertical space. Inspect/zoom affords detail, but this remains a layout readability weakness. Counts need interpretation: projected semantic results can include ports embedded in scene nodes, so a result count and scene-node count are not interchangeable.

The dense parent Diff keeps the original comparison available and adds useful owner groups and review modes. It still has a dense bundle of routes near PlatformArchitecture, and its initial crop/irrelevant retained Inspector were recorded in the earlier product review. The candidate Diff is much clearer: the added part and changed owner are visible and the lifecycle strip distinguishes Working review from validation/commit. Those observations do not by themselves qualify arbitrary dense graph interaction.

## Existing adversarial coverage and the missing gate

| Stress case | Existing coverage before this follow-up | Limitation |
| --- | --- | --- |
| Parallel relationships | Distinct-route test with two links; separate-endpoint test with four; visual adversarial fixture adds twelve links. | Did not require each of twenty closely spaced lines to select its own canonical relationship under a normal pointer radius. |
| Dense bipartite / crossings | `dense_ports` cross-links and 80-node/160-edge gutter tests; label-placement test uses twenty horizontal and twenty vertical route obstacles. | No dedicated large bipartite visual review; label absence can be legitimate suppression rather than proof of readability. |
| Inherited relationships | Real inherited `repositoryRevisions` port/Inspector identity checks, effective-feature provenance tests. | Not a many-inherited-relationships density qualification. |
| Self loops | Visible exterior loops, distinct attachment anchors for one/two/five self relationships, semantic identity retained. | Dense twenty-loop native interaction is not established. |
| Cross-container links | Owner perimeter routing and modeled port positions, including obstruction honesty. | Does not establish every possible long route is readable. |
| Long labels | Long Unicode adversarial scene, Inspector bounded-width/three-line gate, label placement against nodes/ports/routes/other labels. | Actual agent scene still truncates some names at 46%; tooltip/Inspector access does not remove that visual cost. |

The substantive missing correctness gate is nearest-route selection. `SpatialIndex::hit_test` previously ordered eligible edges by priority, bounding-box area and semantic ID. Orthogonal segment boxes have zero area. At twenty parallel node attachments, adjacent routes are only about four world units apart, so a normal six-pixel pointer radius contains several routes. The lower-ID neighbor could win even when the pointer was directly on another visible line. This is a concrete inspection-trust defect: a readable highlighted path is not sufficient if clicking it can inspect another relationship.

The bounded correction compares segment distance within the existing port/node/edge precedence, retaining semantic identity as the final deterministic tie-break. A new twenty-route regression uses actual scene routing, twenty distinct relationship identities, five zoom-dependent hit tolerances and three pointer offsets, then reverses input ordering and verifies identical geometry/hits. Node selection priority remains required. This is presentation/interaction coverage and does not claim language-semantic acceptance or a successful native stress run.

## Requirement category correction

The actual `04a-requirement-neighborhood` PNG labels `subjectWorkspace` and `preservesPriorState` as `ELEMENT`, with `?` explorer icons. Its JSON provides exact known public kinds:

| Element | Canonical ID | Existing kind | Corrected presentation |
| --- | --- | --- | --- |
| `subjectWorkspace` | `fe4dc265-e694-5368-9db2-0d784c9009d5` | `ReferenceUsage` | REFERENCE card, reference icon, Reference Inspector label |
| `preservesPriorState` | `505d0ade-97ec-5940-a1dc-62dbf7484106` | `ConstraintUsage` | CONSTRAINT card, constraint icon, Constraint Inspector label |

Only these exact kinds receive the new visual categories. Unknown kinds remain generic; no suffix matching is used. Constraint remains distinct from Requirement and does not assert evaluation, satisfaction or verification. No semantic records, relationships, requirement-role grouping or durable state change.

Largest criticism, change and remaining weakness: clicking a dense route could identify the wrong relationship; nearest-route selection and a twenty-route gate now address that cause in source. Compilation/regression and native recapture remain pending. Broad dense layout readability, compact agent fitting and many-inherited/bipartite native qualification remain open; no fixture-only Alpha claim is made.
