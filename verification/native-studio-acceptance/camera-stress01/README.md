# First integrated 10,000-node camera run

Passed on the actual native Vulkan renderer with the release executable identified
in `../native-build-01.json`. The deterministic input driver performed 120 zoom
input samples after steady and pan phases. No anchor divergence was observed;
maximum anchor error was 0.002014 world units, below the 0.25-world-unit scenario
limit. The tighter randomized camera/egui tests are separate evidence.

This first run overlapped the workspace test suite. Its retained frame timings
are observations under load, not a clean before/after performance comparison.
The fixture intentionally isolates camera/input behavior; real semantic product
acceptance still requires the self-model journey and soak.

The exact launch, executable digest, process memory and exit status are retained
in `process.json`; `journey.json` retains every anchor trace, including incoming
OS events, viewport, DPI, camera, frame and scene generation.
