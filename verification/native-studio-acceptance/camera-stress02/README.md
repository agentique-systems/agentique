# Fresh-process camera follow-up on the corrected release

All three actual native stress processes passed, including the first 10k run.
Source `42e7e187de823a3b855d530dcee84dbe8c06b59f`; executable SHA-256
`c2d023c62b80c89791a6a1d4ea11626b86b46c87b51812dedd2034ccefddfa5a`.
The retained manifest binds original reports, commands and process measurements.
Each run used the RTX 3060 Ti Vulkan backend with vsync. Compilation had completed
and no other agent semantic workload was running locally.

| Scene/process | Steady median | Pan p95 | Zoom p95 | Maximum anchor error |
| --- | ---: | ---: | ---: | ---: |
| 10k first | 6.030 ms | 16.467 ms | 16.018 ms | 0.002014 world units |
| 10k repeat | 6.066 ms | 14.788 ms | 14.644 ms | 0.002014 world units |
| 1k | 6.047 ms | 6.287 ms | 6.439 ms | 0.001343 world units |

These are phase-specific native frame intervals: 60 warmup frames followed by
120 steady, 120 pan and 120 zoom samples. The reports retain every zoom input and
the unchanged numeric acceptance threshold. There was no first divergent anchor
frame in any run. All timestamp diagnostic error categories were zero.

The two 10k runs are 240 additional real-window zoom frames, not thousands of
independent processes. Separate ordered-event and camera property suites provide
the larger deterministic stress population. These results do not imply physical
input-to-photon timing, manual mixed-DPI qualification or hardware device recovery.

Steady rendering remains near the supplied 165 Hz baseline. The repeat pan/zoom
p95 values are higher than the supplied approximately 12.5/11.4 ms observations;
this follow-up establishes anchor correctness, not a renderer speed improvement.
