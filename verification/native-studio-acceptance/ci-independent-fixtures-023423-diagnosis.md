# Independent fixtures CI diagnosis

CI run [36233529549](https://github.com/agentique-systems/agentique/actions/runs/36233529549), source `023423a743a66b92d2126ab8a2740f3c7d273163`, has the existing Gen1 external-validator disagreement. It does not reveal a new harness defect. No fixture, semantic expectation, verification exit rule, publication freshness pin or release obligation was changed.

The complete `verification-evidence` artifact (ID `10903946497`, 189,015,207 archive bytes) was downloaded successfully into ignored `verification/generated/native-studio-acceptance/ci-36233529549`. The actual CI `verification/independent-fixtures.json` exactly matches the repository's retained diagnostic JSON:

- `sysml-validate` 0.43.1: one error, zero warnings, one hint; exit 1 in 5,670 ms.
- `tests/fixtures/Controller.sysml`, zero-based line 20, characters 72–73: `RES001`, “Feature path segment 'r' does not exist on 'the current scope'.”
- The guarded transition uses the named payload `r : Report` and reads `r.count`.
- The same CI run's official SysML pilot 0.59.0 with `CheckMode.ALL` reports zero errors and two already documented warnings; exit 0 in 10,014 ms. The warnings concern inherited `accepted` names in starter models, not Controller.

All three independently checked fixture hashes match the exact CI commit objects and remain byte-for-byte unchanged in current committed source: Controller.sysml, Parallel.sysml and Types.kerml. Artifact file hashes, full external diagnostic, exact remote commands/exit codes and pilot identities are retained in [ci-independent-fixtures-023423-diagnosis.json](ci-independent-fixtures-023423-diagnosis.json).

The obligation is already explicit in `standards/coverage.json` (Scalar attributes and expressions), `docs/standards-discrepancies.md` (named transition payload rule), `tools/report.mjs`, and the last comment in `.github/workflows/ci.yml`. `tools/verify.mjs` reports the optional diagnostic summary excluding the known disagreement separately, while the actual overall result still requires every command to exit 0. Thus even after unrelated freshness failures are resolved, this intentionally retained npm disagreement prevents claiming a completely green normal verification run.

Gen1's `independent_model_exact_payload_guard` verifies the bounded named-payload behavior. Parallel.sysml is valid SysML representing parallel states outside AGQ-SEQ-01; the Gen1 `at_std02_parallel_preserved` test preserves its source and refuses unsupported execution. Neither is an invalid fixture to remove or rewrite. Generation-2 Alpha progress does not discharge Gen1's incomplete release obligations in `verification/traceability.json`.

No heavy local build was run. The artifact inspection script exited 0 after asserting exact fixture hashes, unchanged sources, identical retained diagnostics, and the same-run pilot's zero-error result. Raw remote outputs remain in the isolated downloaded artifact; the original CI job still correctly reports failure.
