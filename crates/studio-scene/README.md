# agq-studio-scene

The Surface's scene for the Studio: layout, edge routing, hit testing and
culling. It turns a small description of what to draw into positioned cards,
ports and routed edges. Nothing here changes the model; geometry, collapsed
containers, focus, camera and change marks are presentation only.

## Input

`SceneInput::from_tree(tree, locks, problems, generation)` builds the input
from an `agq-language` element tree:

- **Cards** (`InputNode`): packages, definitions, parts, items, attributes and
  requirements, with their owner, lock and problem count.
- **Ports** are shown on the card of their owner and, found through the type
  (never copied), on the cards of usages typed by it. A port shown on a card
  is identified by the pair (card, port): `SceneTarget::Port(card, port)`.
- **Edges** (`InputEdge`): connections, interfaces, satisfy relationships,
  subjects, typing and specialisation. Each end is a card or a port on a card
  (`InputEnd`).

`requirements_view()` keeps requirements, what satisfies them and their
subjects, without nesting. `fixtures` holds inputs for tests, benchmarks and
the Studio's `--fixture` option; the architecture fixture is the Scenario A
URL shortener.

## Layouts

`Scene::build(input, options, previous_memory)` picks the layout from
`SceneOptions::layout`:

- `Hierarchy`: containers hold what they own. Cards keep their previous
  positions (from `LayoutMemory`) when they still fit, so local edits do not
  move unrelated cards. Collapsed containers show the ports of hidden cards
  that connect outside as boundary ports with their real identity.
- `Graph`: layered by the edges; directed cycles become compact groups.
  Positions can be pinned with `LayoutMemory::pin`; overlapping pins are
  reported as `SceneError::GraphPin(PinError::Conflict { .. })` instead of
  being drawn on top of each other.
- `Requirements`: lanes for requirements, what satisfies them and subjects.

`LayoutMemory` is saved with the session, never with the model.

`Scene::comparison(before, after, changed, options, previous_memory)` marks
added, changed and removed cards, ports and edges against an earlier input;
removed ones stay as ghosts where they were.

## Updates after an edit

`scene.update(input, options, previous_memory)` makes the scene for an edited
input from the current scene (S5.1, R-28). Cards are placed exactly as
`Scene::build` places them with that memory: the layout is cheap (about 10 to
20 ms at 10k elements) and keeps unrelated cards still. Routing is not cheap
(about 2 s at 10k), so an edge keeps its earlier route when its ends and lane
are unchanged and the route crosses neither the old nor the new place of a
card that was added, removed, moved or resized. Every other edge is routed
again. `Scene::routing()` says how many edges were routed and how many kept.

One deliberate difference from a build: a kept route can keep a detour that a
build would no longer choose, around a card that has since moved or gone.
Routes stay put, like cards; a build routes them all again.

## Routing, hit testing and culling

Routes are orthogonal, start and end exactly at ports, keep parallel edges in
separate lanes and loop around their card for self edges. When the bounded
search finds no clear corridor the route is marked `RouteQuality::Obstructed`.

`SpatialIndex` answers hit tests (ports first, then cards, then edges, then
container backgrounds), marquee selection and visible-area queries.
`SceneLookup` finds records by identity. Both belong to one scene
`generation` and return nothing for another. Geometry is in world units; hit
tolerance should be `pixels / zoom`.

```powershell
cargo test -p agq-studio-scene
cargo test --release -p agq-studio-scene --test budgets -- --nocapture --test-threads=1
cargo run --release -p agq-studio-scene --example scene_benchmark
cargo run --release -p agq-studio-scene --example edit_benchmark
cargo run --release -p agq-studio-scene --example layout_quality
```

The benchmarks measure CPU work only (layout, routing, indexing, hits and
culling); GPU upload and frame timing are measured by the Studio.
`edit_benchmark` compares a full build with an update for five kinds of edit
at 1k and 10k elements, in both layouts.
