# Interaction reliability work

The previous first 10k benchmark failed with 1042.277466 world units of pointer-anchor error; its unchanged repeat passed. Retained aggregates do not prove the exact events in that historical failure.

The production viewport applied egui's smoothed scroll delta at the *current* pointer, including later pointer moves during its residual scrolling tail. It also reduced multiple events in a frame to a single final hover position. The replacement consumes wheel events in order, at each event's pointer, once per egui frame. It uses logical viewport coordinates and rejects nonfinite camera input. There is no deferred toolkit wheel motion to retarget. Hit testing now follows the resulting camera update. Native stress assertions are unchanged; its report additionally retains event provenance, viewport/DPI/camera state, and first divergence.

Added verification (execution pending integration on the shared build target): legacy reproduction using the former production path versus the corrected path with identical actual egui events; 12,000 native event-order/DPI/cadence cycles; 100,000 camera cases at multiple scene scales; mixed-DPI node/port/popup coordinate checks; full Back/Forward camera/selection/filter restoration; bounded projection-read mailbox and stale-reply tests.

Navigation now snapshots complete disposable presentation per visit, including revision-scoped selection, filters, layout and panel state. Updating the restored visit does not discard Forward history. Saved selections are reconciled against the arriving exact revision and scene. The immutable read lane has one latest projection request slot alongside panel reads; a current-revision view can be constructed while the serialized mutation worker prepares a candidate. Candidate review and historical revisions keep their existing authority paths. Replies still require matching epoch, revision, request and exact view definition.

Unrecoverable device loss already reports an actionable OS title/stderr message and disables blind editing. The first detected fault now attempts immediate presentation persistence rather than waiting for the eight-second periodic-save interval. Actual GPU device loss and manual mixed-monitor interaction are not qualified by unit checks.

Commands already executed in this worktree:

| Command | Output | Exit |
| --- | --- | --- |
| `cargo fmt --manifest-path crates/studio-native/Cargo.toml --all` | empty | 0 |
| `cargo fmt --all` | empty | 0 |
| `git diff --check` | empty after formatting | 0 |

Compilation and test execution are coordinated on the integration checkout to reuse existing build artifacts; this machine had about 1.1 GiB free during implementation. No passing tests or benchmark improvements are claimed by this record.
