# Graph positional pins

Implementation base: `96b9de4`; isolated branch `work/alpha-product-pins`.
Scope is the disposable scene crate. No native rendering, language producer,
canonical model, revision, repository or provider code changed.

The new serde-default `LayoutMemory.pinned` map stores exact top-left graph
positions by ElementId, separately from mutable layout hints. API:

```rust
memory.pin(element_id, node.bounds)?;
let pinned = memory.is_pinned(element_id);
let removed = memory.unpin(element_id);
```

`pin` validates finite positive bounds before changing presentation memory. The
next graph projection lays out active pins first, moves conflicting unpinned
nodes aside, and packs additional nodes around them. The card dimensions still
come from the current projection. Hidden and stale IDs are retained for possible
return but reserve no space. Hierarchy layout ignores the pin constraints.

Two overlapping active pins cannot both satisfy a non-overlap constraint.
`SceneError::GraphPin(PinError::Conflict { first, second })` returns exact
identities rather than silently moving a pinned object or exposing a successful
overlapping scene. The native caller should preserve the previous scene and
let the operator unpin a conflicting item. Semantic changes that enlarge two
pinned cards can trigger the same explicit presentation error. This is neither
a model diagnostic nor a semantic candidate failure.

Seven focused tests cover local expansion and revision change, pinned precedence
over retained unpinned geometry, unchanged semantic records, hidden/stale ID
recovery, explicit collision and unpin recovery, card growth, backward-compatible
session deserialization, saved-anchor round trips, hierarchy isolation, and
invalid geometry. The scene's existing 29 contract tests and one unit test pass.
Actual native pin/unpin interaction and screenshot acceptance belong to the
integration gate; these fixtures make no real-model acceptance claim.

| Command | Output | Exit |
|---|---|---|
| `cargo fmt --all` | no output | 0 |
| `cargo test --locked --offline -j 2 -p agq-studio-scene` | `graph-pins-tests.txt`: 37 tests passed | 0 |
| `cargo clippy --locked --offline -j 2 -p agq-studio-scene --all-targets -- -D warnings` | `graph-pins-clippy.txt` | 0 |

No native build or concurrent broad build was started for this bounded batch.
