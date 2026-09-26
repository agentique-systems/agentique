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

## Real soak and Rename gate

`--scenario real-restart --soak-seconds 600` first executes the existing separate-process restoration gate, then repeats real navigation, camera, Inspector, derived Explain, requirements, agent, history and parent-diff input until at least ten minutes of soak cycles have completed. Each cycle cancels a Rename during preparation after observing a current Graph projection complete while mutation remains pending, then constructs another identity-preserving Rename, validates and cancels it. All editing uses normal widgets; the driver never invokes a platform mutation directly. The durable committed head and persisted part identity must remain unchanged. The first cycle retains requirements, dense diff and Rename captures.

The external `run_real_soak.py` runner records native stdout/stderr, exact command, executable SHA-256, exit code, and process/tree memory at 250 ms intervals. It identifies the first report-observed soak sample separately from cold startup and retains start/end/peak RSS. Example (use fresh output paths):

```powershell
python verification/native-studio-acceptance/run_real_soak.py --native target/native-alpha/release/agq-studio-native.exe --root . --runtime-dir C:/Users/phili/.agentique --database <absolute-accepted-database> --restart-report <successful-real-journey.json> --output <fresh-soak-directory> --seconds 600 --timeout 3600
```

Additional commands executed: `python verification/native-studio-acceptance/run_real_soak.py --help` printed its argument list, exit 0; `python -m py_compile verification/native-studio-acceptance/run_real_soak.py` produced no output, exit 0. These are script checks, not a completed soak. The native soak requires integrated compilation and an accepted real first-process journey.

## Independent retained-context review

Reviewed root commit `e1163c7332b91ce55ef3623c509ad79584178583` for authentication, lifetime, concurrency and cache invalidation. No blocker found: retained KerML context copies all checked context fields and owns immutable Arc/COW graph/index storage; borrowed evaluator lifetimes remain tied to that owner. Retained SysML also binds composed identity and standard bindings. `SourceCompilation` initializes after its final graph frontier and has no mutation API that invalidates its OnceLocks; attaching the effective audit afterward does not alter the graph. Fresh constructors do not recurse into these locks. Each read still constructs independent bounded evaluator caches. Actual naming extension is stateless. This review does not substitute for cold/incremental oracle equivalence or concurrent first-use tests.
