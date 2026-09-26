# CI review: source 42e7e187

[Run 36236540319](https://github.com/agentique-systems/agentique/actions/runs/36236540319) completed with **failure**. No additional failing stage or diagnostic was found beyond the known publication freshness checks and generation-1 named-payload validator disagreement.

Workspace Rust checks passed: formatting, Clippy with warnings denied, build and 1,165 test passes (14 ignored). Linux native formatting/Clippy and 158 tests passed; macOS native compilation passed. Frontend checks/build, eight browser tests, process recovery and the headless demo passed.

The Studio runtime browser gate passed setup/fail-closed coverage, but skipped its real accepted-runtime journey. No interactive Linux/macOS qualification or real runtime gate is inferred from this run.

| Verification stage | Exit | Duration (s) |
|---|---:|---:|
| registers | 0 | 0.090 |
| extraction | 0 | 0.034 |
| standards | 1 | 0.193 |
| metamodel | 0 | 18.409 |
| rust-format | 0 | 2.785 |
| rust-clippy | 0 | 55.684 |
| rust-tests | 0 | 1206.992 |
| rust-build | 0 | 6.491 |
| frontend-format | 0 | 1.809 |
| frontend-types | 0 | 2.226 |
| frontend-build | 0 | 1.557 |
| engineering-tests | 1 | 0.612 |
| independent-starter | 0 | 5.814 |
| independent-fixtures | 1 | 4.800 |
| official-pilot | 0 | 9.039 |
| browser-tests | 0 | 61.939 |
| studio-runtime-setup | 0 | 55.279 |
| process-recovery | 0 | 2.308 |
| headless-demo | 0 | 1.474 |
| update-registers | 0 | 0.097 |

The three nonzero stages are:

- `standards`: changed implementation/source pins fail `verifySystemsPublicationFreshness`. Pins remain unchanged pending the final exact semantic oracle.
- `engineering-tests`: 25 passed, one failed. The only failure is `SysML normative inputs and exact archive dependency closure verify offline`, at the same Systems freshness comparison.
- `independent-fixtures`: exactly the previous `RES001` on the named accept payload `r` in `tests/fixtures/Controller.sysml` (zero-based line 20, character 72). The JSON is byte-identical to the prior retained diagnostic (SHA-256 `99a52db50c41a7911047b8a5a824943f3dd9bbab8cfb4dae8da1d4c974660df5`), and fixture hashes match. The official pilot passes with zero errors and the same two known warnings.

Full commands, durations, exits, environment, counts and retained-file hashes are in `checks.summary.json`; exact upstream records are in `checks.results.json`. Raw stage output is retained as `checks.logs.*.txt`, alongside the full job log. The 256,170,832-byte verification ZIP was downloaded to the ignored isolated directory and matched the GitHub artifact SHA-256 `5a628cdc10427b51c63944774d944bf64bc0207c845980a20cbf571be4fe35cf` before bounded extraction.

No production code, fixture, acceptance obligation or freshness check was changed. No local Cargo command was run. These results qualify source `42e7e187de823a3b855d530dcee84dbe8c06b59f`; later source needs its own checks. Existing `linux-native.*`, `macos-compile.*`, and `result.json` files were left unchanged.
