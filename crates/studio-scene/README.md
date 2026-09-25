# Agentique native scene

`agq-studio-scene` converts an actual `agq-modeling-view::ViewProjection` into
disposable geometry. It owns no semantic truth, repository storage, language
producer or toolkit. It retains immutable revision/element/relationship IDs.

The native shell supplies a projection, `SceneOptions` and optional
`LayoutMemory`; a renderer consumes retained nodes, semantic ports and routed
edge polylines. Layout memory, camera, LOD, selection and overlays are session
state. Structural containment is a presentation of existing owners; Graph World
uses SCC-condensed topology without changing those owners. Layout preserves old
positions when geometry permits and retains hidden positions through collapse.

The uniform spatial grid serves hits, marquee and culling. Very large containers
and edge segments use an overflow list rather than allocating arbitrarily many
cells. Port hits precede nodes, then edges, then containing backgrounds. Geometry
is logical world coordinates; hit tolerance should be `logical_pixels / zoom`.

Routing is deterministic orthogonal corridor search with indexed obstacles,
semantic endpoint ports, parallel lanes and self loops. Bounded search reports
`RouteQuality::Obstructed` explicitly when it cannot find a clean route. It is
not a claim of globally optimal edge routing. Collapsed endpoints are omitted
instead of being silently retargeted to a different semantic identity.

The current view transport exposes metaclass names. One exact-name adapter
assigns a typed `NodeCategory`; no name substring or element label heuristic is
allowed. Unknown metaclasses remain generic. Port direction stays `Unspecified`
until a public semantic projection carries an authoritative direction.

`fixtures` contains explicitly labeled visual test records for architecture,
ports, requirements, comparison and 1k/10k scale. These use the real DTO shape but
do not establish language acceptance, candidate validity or durable history.

```powershell
cargo test -p agq-studio-scene
cargo run --release -p agq-studio-scene --example scene_benchmark
cargo doc -p agq-studio-scene --no-deps
```

CPU benchmarks distinguish projection adaptation, layout, total scene build,
spatial indexing, hits and culling. GPU upload, frame timing and input-to-frame
latency must be measured by the native renderer; camera arithmetic is not an
input latency result.
