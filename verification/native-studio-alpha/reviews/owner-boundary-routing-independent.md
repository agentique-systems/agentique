# Independent review: owner-boundary routes and port headers

Reviewed together: `df2c49240e882f296b35335a279c22ee9bdaa1a7` and correction
`1223754168ee849c9a6ec221ca97a6c62ffc8cef`.

This is an independent source review, not a build, runtime, or screenshot
acceptance. No standards consumer, application process, or test was launched.
The actual motivating image remains [real-run04 selected Platform port](../real-run04/gallery/02a-platform-port.png);
the reviewer previously inspected that image and its canonical sidecar during
the [real-run04 review](real-run04-editor-interaction.md). A new real image must
show whether these changes resolve the misleading visual ownership.

## Judgment

No source-level blocking authority or geometry defect found in the final pair.
The correction fixes both the misplaced `side` initializer and the unequal-stub
loop diagonal identified during integration. Source approval is conditional on
focused execution and the next actual Platform/port capture; it is not an alpha
product acceptance.

## Ranked remaining limitations

1. **P2 — Exterior labels can still cover adjacent cards.**
   `studio-native/src/viewport.rs:615–665` reserves a bounded internal slot for a
   small port set, but the dense-port/low-zoom fallback places a label outside
   its owner with no neighboring-card collision query. The allowed width can
   reach 44% of the owner width. At scale 1, a right-hand label wider than the
   hierarchy's 44-world-unit gutter can cover the adjacent type or subsystem
   card. The leader preserves the correct port association, and this avoids
   covering this owner's children, but does not establish global label
   clearance. The actual two-port Platform uses the internal header path and
   does not require this fallback at the prior approximately 71% zoom. Retain
   this as a visual qualification item for dense ports and low zoom; do not
   claim arbitrary label collision avoidance.
2. **P2 — Port-count transitions do not yet have a displacement gate.**
   `studio-scene/src/layout.rs:362–365,449–460` clamps retained child origins to
   the enlarged header, then uses the existing collision-aware sibling pack.
   This prevents child overlap, but changes such as no ports to one port, or
   two to three ports, can move children and enlarge the owner. The new scene
   contract verifies exactly unchanged geometry on the next rebuild for an
   unchanged port set. It does not measure the unchanged-node displacement
   distribution after adding a port. Test that transition separately before
   claiming preservation of the mental map across this edit class. Small
   movement needed to reserve a legible header is a reasonable tradeoff.
3. **P2 — Actual product verification is outstanding.** The new tests are
   meaningful source-level cases; this reviewer did not execute them. The real
   after-image must include both Platform ports, their connection and the
   contained components at normal operator zoom. Check that the exterior
   connection remains individually traceable and does not force an unreadable
   fit. An unchanged fixture alone cannot close the run04 criticism.

## Invariants traced

- `routing.rs:154–168` obtains a port's exact position and visual side from the
  `ScenePort`, resolves the exact visible owner bounds, and does not fabricate
  direction. `route_edges:75–80` retains the entire original `ViewEdge` DTO.
  Canonical source, target, relationship ID, origin, rule and revision remain
  unchanged. Ordinary attachment lanes remain presentation geometry and do
  not introduce semantic ports.
- Equal visible owners use `route_around_owner:271–309`, trying at most four
  clearances in both directions. The perimeter starts and ends at the exact
  boundary points. Other leaf/collapsed card obstacles are still inspected;
  unavoidable collisions remain `RouteQuality::Obstructed`. Neither endpoint
  ownership nor a successful route is used as semantic compatibility evidence.
- `owner_perimeter:342–398` traverses a rectangle outside the owner bounds;
  corner ordering covers both directions without crossing the owner interior.
  `owner_loop:311–339` now derives both actual stub lengths, expands beyond the
  larger one, and returns through the target stub. This fixes the diagonal
  possible with staggered generic node attachments. The added source tests
  cover all boundary-side pairs, self-loops, unequal stubs, parallel routes,
  and unavoidable obstruction while checking exact endpoints and orthogonality.
- `layout.rs:165–186` deduplicates actual projected port identities from records,
  feature summaries and collapsed proxies. The new reserved header uses that
  count, rather than an advisory feature count. Reservation is capped at two
  rows: 92 or 116 world units. Larger sets retain the existing bounded header.
- `lib.rs:299–351` checks actual child geometry before enabling internal port
  labels. `layout.rs:449–507` still collision-checks retained and newly allocated
  siblings; increasing the minimum header does not simply draw over the old
  first row. The exact `ScenePort` identity, revision, owner/proxy and unspecified
  semantic direction are retained. Existing canonical metadata is not rewritten.
- `viewport.rs:620–644` bounds opposing label widths and requires enough screen
  height for each reserved row. At insufficient zoom it chooses the explicit
  leader fallback. The marker remains at the semantic port's visual attachment;
  label placement does not become an alternate selectable semantic object.

The added per-scene maps/sets are bounded by projected nodes and distinct ports;
the owner router evaluates a fixed number of candidates per edge. No new global
cache, retained semantic context, unbounded retry, or publication authority path
was introduced. This is a source complexity assessment, not a frame-time or
memory measurement.
