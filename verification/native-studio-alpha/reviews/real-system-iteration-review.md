# Real product iteration: readable systems, unfinished reasoning views

**Systems engineer / graph critic judgment: the System views now communicate
useful architecture, but this build is not ready for a confident complete product
demo.** This review visually inspected all ten retained `real-run03` images.
It did not substitute successful semantic assertions for visual acceptance.

The before images and original ranked criticism are in the
[first real visual review](real-system-first-light-review.md). Both runs use the
actual Agentique model and accepted KerML v9 / Systems v3, on revision
`8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46`. The ten new image hashes independently
match their native scenario records; each record explicitly identifies real
semantic data and has no fixture. Exact hashes and sizes are in the
[image audit](real-system-iteration-images.json).

## Changes that made a visible difference

Commit `4139324db8ac1da942a00d56142ca6314a21c734` introduced a one-level fresh
System view, compact root packing, labels at distant zoom, a root-first Explorer,
actual initial root inspection and identity-checked Explorer double-clicks.
Commit `76f1198e757e62242bf67e651af9eda90b38d82d` then retained layout coordinate
systems referenced by Back/Forward and each world's Home, correcting the reviewed
key-order eviction problem. Commit `5ceb224cb524d9892f563e1ab4a8560bcbfec7a2`
made named project features primary in the Inspector while retaining standard,
derived and generated facts behind disclosure.

| Engineering question | Before | New image and observed result | Score / 5 |
| --- | --- | --- | --- |
| What does Agentique contain? | 38 records at 23%; mostly anonymous boxes; root buried in Explorer | [System](../real-run03/gallery/01-system-world.png): 7 nodes at 119%; Agentique and its three parts are readable; actual root is selected | 4 |
| Can I enter ModelingPlatform? | 32 scene nodes at 24%; subsystem remained tiny | [Focused subsystem](../real-run03/gallery/02-focused-subsystem.png): 20 projected / 18 scene nodes at 71%; named parts inside a clear boundary | 4 |
| What is this engineering object? | Repeated generic Connector and standard feature entries preceded useful facts | Focused subsystem Inspector shows authored parts, named ports/interfaces and behavior; exact effective facts remain available | 4 |
| Can I understand an inherited port? | Inheritance was not visually qualified in the earlier gallery | [Repository focus](../real-run03/gallery/02b-focused-repository.png) and [selected port](../real-run03/gallery/02c-repository-port.png): 5 projected / 4 scene nodes at 119%; Repository owns repositoryRevisions; Inspector states inherited origin and ModelRevision type | 4 |
| Can I identify the connected platform ports? | Selection had no useful spatial identity | [Selected clientQueries](../real-run03/gallery/02a-platform-port.png): useful connection Inspector, but selected endpoint name is still absent at 71% | 2 |
| Can I reason about dependencies? | No equivalent real gallery | [Graph](../real-run03/gallery/03-graph-world.png) and [agent view](../real-run03/gallery/07-agent-view.png): 19 projected / 18 scene nodes at 42%; many names truncated and broad package context dominates | 2 |
| Can I follow a requirement into architecture? | Literal subject-to-type path was missing from the view | [Requirements](../real-run03/gallery/04-requirements-world.png): 6 nodes at 73%; ImmutableRevisions, subjectWorkspace and ProjectWorkspace retain their separate subject/typing path | 3 |
| Can I see why a derived relationship exists? | No equivalent real gallery | [Explain](../real-run03/gallery/05-explain.png): real evidence appears, but rule and conclusion are below the visible diagram clip | 1 |
| Can I understand a revision change? | No equivalent real gallery | [History diff](../real-run03/gallery/06-history-diff.png): 159 projected / 130 scene nodes at 8%; additions are an anonymous horizontal strip | 1 |

These are qualitative review scores, not measured usability results. The clearer
System and inherited-port views approach alpha quality; Graph, Explain and the
default diff presentation remain foundation quality.

## Ranked remaining problems and requested next iteration

1. **P1: comparison does not explain the change by default.** The diff's
   `+17 added` header and Focus changes button are useful controls, but its initial
   camera makes every engineering name unreadable. Enter comparison around the
   affected authored owners or changed region; keep the full graph accessible.
   Preserve exact revisions and unchanged positions when adjusting presentation.
2. **P1: the selected platform port has no readable endpoint label.** Its cyan
   route passes through child cards, and neither the selected nor opposite port
   is easy to locate. Selected and directly connected endpoint labels should
   survive the current detail level. The Inspector's real `queryConnection`
   answer is necessary semantic evidence, but does not repair the canvas.
3. **P1: a dependency request expands through a package into siblings.** Source
   inspection confirms `StudioPlatform::dependencies` uses ordinary undirected
   `projection::neighborhood`. Two ownership hops reach PlatformArchitecture,
   then ViewService, Client and other package siblings. This is valid graph
   adjacency, but weak engineering dependency communication. For an explicit
   dependency scope, make a reached canonical Package terminal for outgoing
   Ownership unless that Package was the original seed. Preserve its incoming
   owner anchor, real non-Ownership relationships, and Type/Feature ownership.
   Do not apply a blanket Namespace exclusion: Types are Namespaces. Keep
   ordinary Graph exploration available and label the result's limited scope.
4. **P1: Explain's causal conclusion is clipped.** Evidence cards consume the
   visible height while the rule and conclusion lie below it. A bounded summary
   diagram must show evidence, rule and conclusion together before expansion.
   The technical rule identifier in prose also needs a human-facing description
   without losing the exact rule in Evidence.
5. **P2: relationship emphasis still competes with containment.** System owns
   edges duplicate already clear boundaries and cross typing paths. Requirements
   shows a useful literal subject/type path, but owns and derived labels obscure
   it. Preserve canonical inspectability while giving each world deliberate
   relationship emphasis. Do not remove these facts from semantic projections.
6. **P2: context restoration remains confusing.** ModelRepository search survives
   into Requirements and diff, leaving the Explorer empty. The repository trail
   shifts toward PlatformArchitecture, which is a truthful canonical owner but
   differs from the operator's entry path through ModelingPlatform. The final
   return-to-System assertion records 37 projected / 34 scene nodes at 28.46%,
   rather than the initial 7-node view. This is an observed state regression;
   shared exploration depth is a source hypothesis requiring confirmation.
7. **P2: engineering summaries need editorial consistency.** ModelingPlatform
   says `9 parts` while showing eight PART cards and an INTERFACE; the repository
   says `1 parts` and `1 ports`. Use the same category vocabulary as the cards,
   even where the canonical language hierarchy has broader inheritance.

The agent screenshot still contains the old illustrative `0.72` decision card.
The independently reviewed real MockViewDecision integration in `ba770545` was
not in this capture. Its source review does not qualify its eventual rendering;
retain a new actual screenshot after integration.

## Journey and qualification limit

The [real-run03 report](../real-run03/journey.json) ends **failed**, with
`Native pan did not move the current camera during background preparation`.
It records five background frames, no observed pan, no completed current-world
inspection during preparation, and no committed candidate. No `08-candidate`
image exists in this run. This review therefore cannot qualify background
responsiveness, Candidate World, commit or durable restart.

The actual journey does establish navigation, canonical inherited-port inspection,
the real connector endpoints, derived Explain evidence, the literal requirement
path and a revision-bound comparison before that failure. Its stage durations
include scenario waits and multiple operations; they are not isolated query or
input latency measurements. No new build or runtime consumer was launched for
this review. Parent-owned fixes following this criticism require another real
capture and review before their remaining weaknesses can be marked resolved.
