# Agentique Native Studio

Native Studio is the Rust spatial product foundation. The browser Studio is
**Agentique Studio Prototype 0**, retained as an interaction reference, API client,
browser compatibility surface and regression client.

```powershell
cargo fetch --locked --manifest-path crates/studio-native/Cargo.toml
cargo run --release --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
# Explicit rendering/interaction fixtures, without accepted semantic runtime:
cargo run --release --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target -- --fixture architecture --no-restore
```

The first build requires fetching the native Cargo.lock dependencies once. No build or
startup action acquires or republishes standards. The setup surface accepts a local
authenticated `.agq-runtime` bundle through the existing runtime-publications
package. Once installed, choose a project; subsequent launches restore the saved
project/revision and presentation session. A missing runtime never silently becomes
a fixture. Fixtures are opened explicitly and remain labeled in the title surface,
viewport, inspector and candidate controls.

## Create a project and add source

After runtime authentication, open the project menu and choose **New project**.
Enter a name and an absolute `.sqlite` repository path, then choose **Create
project**. The resulting project is empty and Working. An existing repository
can hold several projects; selecting a new repository does not delete the old
one.

Choose **Add source document**, enter an absolute local `.sysml` or `.kerml`
file path, and choose **Prepare source candidate**. Review the exact imported
text and its Current/Candidate/Diff views. Validate and commit to advance the
durable head, or cancel to keep the original revision. Invalid source remains
an uncommitted candidate with diagnostics. The imported file's basename is the
document path in the project; a duplicate path is rejected.

Current-world navigation and inspection remain available during candidate
construction. Cancelling prevents its result from becoming the current revision;
the underlying semantic operation may still need to finish before its worker can
start another edit. Runtime authentication and project reconstruction still take
minutes on the measured real model. See the
[current measurements](../verification/native-studio-acceptance/read-profile-02/README.md).

## Architecture

```text
studio-native: chrome, commands, selection, navigation, presentation session
    ├── studio-scene: disposable ViewProjection → layout/routes/spatial index
    ├── retained wgpu instance renderer + toolkit glyph atlas
    └── bounded background worker → studio-platform
                                     ├── modeling-view
                                     ├── modeling-agent
                                     └── ModelingService → repository → workspace
```

The native shell does not speak loopback HTTP, query SQLite tables, invoke producer
scheduler internals or edit canonical records. The shared platform adapter owns
bootstrap and in-process operations. The HTTP and Systems Modeling APIs remain
available for browser/remote/external clients. Generation 1 remains separate.

The outer UI application has its own Cargo workspace and lockfile. Its graphics,
windowing and accessibility dependencies resolve separately from the accepted
language workspace; the reusable scene and platform crates remain root workspace
members. This prevents UI feature unification from changing the accepted language
dependency closure. The freshness gate authenticates the original lock bytes and
requires exact package identities, checksums and dependency edges for all 69
reachable language packages. Every original interpretation input remains pinned.
See [the compatibility proof](../verification/compatibility/README.md).

The native lock still activates additional optional dependencies in shared
third-party packages. Root workspace compatibility does not establish real native
semantic acceptance; that requires the authenticated runtime and self-model gate.
Use `--target-dir target` from the repository root to share build output rather
than creating a second graphics build cache. CI checks the native workspace
explicitly, in addition to the root workspace regression:

```powershell
cargo fmt --manifest-path crates/studio-native/Cargo.toml -- --check
cargo clippy --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target --all-targets -- -D warnings
cargo test --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Scene objects preserve canonical element/relationship identity, presentation
identity and project revision. Layout and animation are disposable. Graph and
Requirements use topology layout; System uses containment. Navigation history is
local camera/focus state; durable history comes from the repository. Removed diff
objects retain their earlier revision for inspection.

The GPU renderer retains four ordered instance ranges: containers, edges, nodes,
and overlays. Camera transforms use a uniform. A changed scene/visibility/selection
uploads new batches; ordinary steady frames reuse the existing buffer. Labels use
the native toolkit atlas, culled before shaping and selected by five hysteretic LOD
levels. This interface can support a future 3D renderer sharing identities and
selection without changing model semantics.

## Interaction

| Gesture | Action |
| --- | --- |
| Left click / Shift-click | Select / toggle additional selection |
| Drag canvas | Pan |
| Shift-drag | Marquee selection |
| Wheel | Zoom toward pointer |
| Double-click / F | Focus selection |
| Home | Fit view |
| Alt-Left / Alt-Right / Alt-Up or Backspace | Back / forward / semantic owner |
| Ctrl/Cmd-Z | Presentation navigation back; never rewrites durable history |
| Up / Down | Traverse semantic elements |
| Ctrl/Cmd-K | Search commands or focus an element |
| 1 / 2 / 3 / 4 | System / Graph / Requirements / History |
| D / E / N | Dependencies / Explain / select neighbors |
| Escape | Dismiss surface, clear selection, then back |

Create nested PartUsage is a typed semantic intent. In a real project the platform
maps it to source, reconstructs a Working candidate, and provides Current,
Candidate and Diff views. Validation uses the existing acceptance contract. Commit
requires an explicit operator command and a durable compare-and-set receipt.
Cancel discards uncommitted work. Durable history is never rewritten by local undo.
Rename Part is also available for a bounded plain authored declaration. It changes
one declared name, preserves the canonical identity, and refuses the candidate if
existing references would change or break. Complex headers and quoted names still
require expert source editing. Both commands capture their target when the dialog
opens; a later selection cannot redirect the edit, and a changed revision requires
reopening the command. Real rename qualification is pending the accepted-runtime
journey; see the [proof and scope](../verification/native-studio-alpha/reviews/part-rename-design.md).
Connection creation and delete remain unsupported.

The fixture candidate is an illustrative display response to the same intent. It
does not reconstruct sources or establish language acceptance. Validation and
commit are disabled. Agent dependency views retain their subject, revision and
read-only authority. The decision panel displays the selected mock provider's
actual returned choice; it shows no weights when the provider supplies none.
Applying that suggestion is an explicit presentation action and cannot commit a
semantic change.

## Reproducible native visual review

```powershell
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target -- --fixture architecture --no-restore --screenshot verification/generated/native-studio/system.png --frames 180 --metrics verification/generated/native-studio/system.json
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target -- --fixture stress1000 --no-restore --frames 360 --metrics verification/generated/native-studio/1k.json
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target -- --fixture stress10000 --no-restore --frames 360 --metrics verification/generated/native-studio/10k.json
cargo run --locked --offline -p agq-studio-scene --example scene_benchmark
```

Other explicit fixtures are `ports`, `requirements`, `typography`, and `diff`. Native screenshots
come from the rendered GPU surface. Frame intervals include vsync and CPU work;
they are not GPU timestamp measurements or physical input latency. CPU scene
benchmarks separately record layout, indexing and hit-testing.

The application uses native accessibility integration for chrome and exposes
semantic names for visible canvas elements. The outliner provides a keyboard
alternative to spatial pointing. High contrast and reduced motion are palette
commands. Full screen-reader, mixed-DPI, IME and complex-script shaping acceptance
remain platform-specific qualification work, documented in ADR 0030. No claim of
full accessibility conformance follows merely from enabling AccessKit.

Session JSON is disposable and versioned. Candidates are process-local and are
never restored as durable model state. Atomic presentation saves are separate from
the modeling service's durable commit protocol.

The alpha iteration's [product language](native-studio-product-language.md),
[eight-surface gallery](../verification/native-studio-alpha/GALLERY.md) and
[real acceptance procedure](../verification/native-studio-alpha/real-acceptance-runner.md)
separate observed native interaction from pending semantic acceptance.
