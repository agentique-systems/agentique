# Native performance extraction

Journey outcome: **failed**.

This report preserves failed/missing stages. It does not establish product acceptance.

Recorded source commit: `7d31b98798190c9de87746d0434110236461e9f8`.
Recorded executable SHA256: `24e57251f274c4b4066e7e5219ddb4e31030ca44d338393f3f24b9e4253dcb83`.

## Observed semantic workflow

| Native stage | Asserted | Whole UI step, ms |
|---|---|---:|
| prepare | True | 143240 |
| validate | not observed | — |
| commit | not observed | — |

elapsed_ms starts when the native step is initialized before input injection and ends at its state assertion. It includes input sequencing, worker queue/work, UI polling/settling and follow-up projections/reads. It is not an isolated prepare/validation/commit engine timer. Missing stages are null; failed stages are not successful timing results.

Clock begins immediately before injected Prepare-button release; ends at the input hook first observing a candidate and no pending mutation. Includes queue, semantic work and native response handling; not a compiler-only timer. Hook gaps are not frame/presentation latency.

Prepare-release to candidate observation: `140216` ms.

## Call samples

| Operation / cache / result | Kind / scope | Revision | n | Min ms | Median ms | Max ms |
|---|---|---|---:|---:|---:|---:|
| inspect / query / ok | Inspector / SelectedElement | `19f46877-550e-4d05-95da-f7b4dcc74e72` | 2 | 2798.913 | 2800.641 | 2802.369 |
| project / query / ok | Architecture / Neighborhood | `19f46877-550e-4d05-95da-f7b4dcc74e72` | 2 | 2921.259 | 2933.399 | 2945.539 |
| inspect / query / ok | Inspector / SelectedElement | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 7 | 2721.771 | 2880.285 | 2932.820 |
| project / query / ok | Architecture / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 6 | 2704.952 | 2896.167 | 2934.749 |
| project / query / ok | Requirements / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2718.800 | 2718.800 | 2718.800 |
| project / query / ok | SemanticGraph / DependencyNeighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2743.714 | 2743.714 | 2743.714 |
| project / query / ok | SemanticGraph / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 6 | 2710.692 | 2723.485 | 2744.774 |
| project / query / ok | SemanticGraph / Neighborhood | `8d7fa8e8-3834-4048-b3bd-e85d1d675f16` | 1 | 2712.067 | 2712.067 | 2712.067 |
| revision_reader_inspect / hit / ok | — / — | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 9 | 0.024 | 0.027 | 0.045 |
| revision_reader_inspect / miss / ok | — / — | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 7 | 2722.973 | 2881.518 | 2934.081 |

Independent call samples grouped by exact binding/operation/outcome. No p95 from these limited calls. Cache misses include query work; do not sum cache and query times. Groups may include different named objects/full views; their distinct counts are explicit. No queue latency is inferred. Full phase records are retained; use audit_view_profile.py for phase-specific analysis.

## Capture windows

Each capture retains its own latest rolling sample window, which may include preceding views. Windows overlap: do not pool, sum, or label them dedicated steady-state scene benchmarks. Scene build/layout values describe the most recent recorded build.

| Capture | Projected N/E | Scene N/E | Build ms | Layout/routing/diff ms | Frame p95 ms | Hit p95 us | Scene GPU median ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 01-system-world | 7/6 | 7/3 | 0.174 | 0.155 | 6.651 | 1.100 | 0.013 |
| 02-focused-subsystem | 20/20 | 18/9 | 0.258 | 0.236 | 6.225 | 1.100 | 0.014 |
| 02a-platform-port | 20/20 | 18/9 | 0.258 | 0.236 | 6.276 | 1.100 | 0.014 |
| 02b-focused-repository | 5/4 | 4/2 | 0.065 | 0.056 | 6.325 | 1.100 | 0.012 |
| 02c-repository-port | 5/4 | 4/2 | 0.065 | 0.056 | 6.331 | 1.100 | 0.004 |
| 03-graph-world | 6/6 | 6/6 | 0.116 | 0.103 | 6.227 | 1.100 | 0.004 |
| 04-requirements-world | 5/5 | 5/5 | 0.081 | 0.072 | 6.532 | 1.200 | 0.002 |
| 05-explain | 7/8 | 7/8 | 0.107 | 0.094 | 6.651 | 1.500 | 0.001 |
| 06-history-diff | 159/425 | 130/405 | 37.170 | 36.760 | 6.492 | 8.200 | 0.025 |
| 07-agent-view | 10/13 | 9/13 | 0.240 | 0.225 | 6.302 | 1.100 | 0.003 |

The JSON retains exact view/revision, source sample counts, all input boundaries, GPU availability/errors, and image verification. No window is pooled with another.

## Limits

- Application frame intervals are UI-update intervals, not measured presentation.
- GPU timestamps bracket only the custom scene pass, excluding text/chrome/upload/present.
- Raw/handled input metrics exclude physical device and OS-delivery latency.
- No submission, presentation, or photon latency is inferred from CPU intervals.
- Native p95 is retained only for windows with at least 20 samples; counts remain explicit.
- No causal speedup or quiet-machine status is inferred from different runs or scene sizes.
