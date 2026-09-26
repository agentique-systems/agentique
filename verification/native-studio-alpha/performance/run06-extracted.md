# Native performance extraction

Input report: `agentique-native-real-acceptance/1` (journey version 1).

Journey outcome: **journey_passed_restart_pending**.

This report preserves failed/missing stages. It does not establish product acceptance.

Recorded source commit: `c9bbc66c4739a6d9d46c6396db26eebc7266c5a9`.
Recorded executable SHA256: `c6a8013dbf7db7103153ca287758390f58e6f1db72e6a57405b2a87c6b5fc57b`.

## Observed semantic workflow

| Native stage | Asserted | Whole UI step, ms |
|---|---|---:|
| prepare | True | 138028 |
| validate | True | 29197 |
| commit | True | 8823 |

elapsed_ms starts when the native step is initialized before input injection and ends at its state assertion. It includes input sequencing, worker queue/work, UI polling/settling and follow-up projections/reads. It is not an isolated prepare/validation/commit engine timer. Missing stages are null; failed stages are not successful timing results.

Clock begins immediately before injected Prepare-button release; ends at the input hook first observing a candidate and no pending mutation. Includes queue, semantic work and native response handling; not a compiler-only timer. Hook gaps are not frame/presentation latency.

Prepare-release to candidate observation: `135029` ms.

## Call samples

| Operation / cache / result | Kind / scope | Revision | n | Min ms | Median ms | Max ms |
|---|---|---|---:|---:|---:|---:|
| inspect / query / ok | Inspector / SelectedElement | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 7 | 2694.484 | 2827.227 | 2917.022 |
| project / query / ok | Architecture / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 3 | 2679.669 | 2836.288 | 2905.366 |
| project / query / ok | Requirements / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2686.998 | 2686.998 | 2686.998 |
| project / query / ok | SemanticGraph / DependencyNeighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2708.090 | 2708.090 | 2708.090 |
| project / query / ok | SemanticGraph / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 5 | 2690.285 | 2694.520 | 2697.183 |
| project / query / ok | SemanticGraph / Neighborhood | `8d7fa8e8-3834-4048-b3bd-e85d1d675f16` | 1 | 2691.677 | 2691.677 | 2691.677 |
| inspect / query / ok | Inspector / SelectedElement | `9539bfed-f725-43d9-b5f8-8254040e7d42` | 7 | 2769.068 | 2820.967 | 2861.168 |
| project / query / ok | Architecture / Neighborhood | `9539bfed-f725-43d9-b5f8-8254040e7d42` | 7 | 2907.822 | 2915.069 | 2945.825 |
| revision_reader_inspect / hit / ok | — / — | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 9 | 0.027 | 0.030 | 0.043 |
| revision_reader_inspect / miss / ok | — / — | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 7 | 2695.682 | 2828.452 | 2918.265 |
| revision_reader_inspect / miss / ok | — / — | `9539bfed-f725-43d9-b5f8-8254040e7d42` | 1 | 2830.725 | 2830.725 | 2830.725 |
| platform_project / hit / ok | Architecture / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 7 | 0.175 | 0.183 | 0.200 |
| platform_project / miss / ok | Architecture / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 3 | 2681.958 | 2837.680 | 2906.794 |
| platform_project / miss / ok | Requirements / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2688.376 | 2688.376 | 2688.376 |
| platform_project / miss / ok | SemanticGraph / DependencyNeighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 2709.427 | 2709.427 | 2709.427 |
| platform_project / hit / ok | SemanticGraph / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 1 | 0.411 | 0.411 | 0.411 |
| platform_project / miss / ok | SemanticGraph / Neighborhood | `62af4537-f2c9-4bf1-90d6-e0862d6b4141` | 4 | 2691.620 | 2694.138 | 2696.920 |
| platform_project / miss / ok | Architecture / Neighborhood | `9539bfed-f725-43d9-b5f8-8254040e7d42` | 1 | 2934.203 | 2934.203 | 2934.203 |

Independent call samples grouped by exact binding/operation/outcome. No p95 from these limited calls. Cache misses include query work; do not sum cache and query times. Groups may include different named objects/full views; their distinct counts are explicit. No queue latency is inferred. Full phase records are retained; use audit_view_profile.py for phase-specific analysis.

## Capture windows

Each capture retains its own latest rolling sample window, which may include preceding views. Windows overlap: do not pool, sum, or label them dedicated steady-state scene benchmarks. Scene build/layout values describe the most recent recorded build.

| Capture | Projected N/E | Scene N/E | Build ms | Layout/routing/diff ms | Frame p95 ms | Hit p95 us | Scene GPU median ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 01-system-world | 7/6 | 7/3 | 0.170 | 0.154 | 6.485 | 0.800 | 0.010 |
| 02-focused-subsystem | 20/20 | 18/9 | 0.264 | 0.236 | 6.563 | 0.800 | 0.012 |
| 02a-platform-port | 20/20 | 18/9 | 0.264 | 0.236 | 6.647 | 0.800 | 0.013 |
| 02b-focused-repository | 5/4 | 4/2 | 0.056 | 0.048 | 6.709 | 0.800 | 0.014 |
| 02c-repository-port | 5/4 | 4/2 | 0.056 | 0.048 | 6.715 | 0.800 | 0.004 |
| 03-graph-world | 6/6 | 6/6 | 0.091 | 0.081 | 6.492 | 0.800 | 0.003 |
| 04-requirements-world | 5/5 | 5/5 | 0.080 | 0.070 | 6.585 | 0.900 | 0.002 |
| 05-explain | 7/8 | 7/8 | 0.110 | 0.097 | 6.670 | 1.300 | 0.002 |
| 06-history-diff | 159/425 | 130/405 | 36.564 | 36.170 | 6.331 | 6.800 | 0.033 |
| 07-agent-view | 10/13 | 9/13 | 0.207 | 0.190 | 6.356 | 0.800 | 0.003 |
| 08-candidate | 21/21 | 19/9 | 0.246 | 0.224 | 6.644 | 1.000 | 0.011 |
| 09-candidate-diff | 21/21 | 19/9 | 0.598 | 0.575 | 6.747 | 1.000 | 0.017 |
| 10-committed-history | 21/21 | 19/9 | 0.232 | 0.210 | 6.270 | 0.800 | 0.018 |

The JSON retains exact view/revision, source sample counts, all input boundaries, GPU availability/errors, and image verification. No window is pooled with another.

## Limits

- Application frame intervals are UI-update intervals, not measured presentation.
- GPU timestamps bracket only the custom scene pass, excluding text/chrome/upload/present.
- Raw/handled input metrics exclude physical device and OS-delivery latency.
- No submission, presentation, or photon latency is inferred from CPU intervals.
- Native p95 is retained only for windows with at least 20 samples; counts remain explicit.
- No causal speedup or quiet-machine status is inferred from different runs or scene sizes.
