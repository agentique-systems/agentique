# Phase 2 acceptance follow-up during Studio Phase 3

Base: fetched `origin/main` at `4ab612e1232cfbff6d879e4b72cba0d2447ee47d`.

All three remaining gates are **blocked by missing accepted publication inputs**:

| Gate | Phase 3 result | Exact ignored test |
| --- | --- | --- |
| Compact semantic-cache restoration and failure authentication | Not run | `durable_cache_and_failure_authentication` |
| Real Gen2 HTTP integration and stable continuation | Not run | `durable_project_revision_http_vertical_and_stable_continuation` |
| 100-document durable semantic scale | Not run | `durable_scale_100_mixed_documents_10_revisions_four_readers` |

The recorded KerML publication cache, Systems publication cache, and retained
`agentique-dogfood.sqlite` from Phase 2 are absent. No cache environment overrides
were present. A bounded filename search covered this checkout, other files under
`C:/Users/phili/github`, `.codex` worktrees, `.workspaces`, Downloads, Desktop, and
Documents; no matching accepted cache or retained self-model database was found.
This is an infrastructure precondition failure, not a new semantic rejection.
Historical Phase 2 evidence remains unchanged. No standard publication was
acquired or rebuilt to bypass the missing inputs.

The actual repeatable preflight command, exit code, source identity and output
hash are recorded in `frontend-commands.json`. Its raw JSON output is ignored
under `verification/generated/agentique-studio-phase3/`:

```powershell
python verification/scripts/studio_artifact_preflight.py
```

To resume, supply existing authenticated publication caches via
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`. Set `AGENTIQUE_SELF_MODEL_DB`
to the retained repository when available. Run the exact gates serially, with
`--release --locked --offline` and `CARGO_PROFILE_RELEASE_LTO=false`; the service
tests also require `--features verification`. Each exact test requires
`-- --exact <test-name> --ignored --nocapture --test-threads=1`. Use the existing
`verification/scripts/watchdog.py` for bounded runtime and recorded exits.
The full-scale gate remains distinct from storage-only benchmarks.

Studio frontend browser fixtures, when run, verify transport and interaction
contracts only. They cannot accept any of the real publication, durable model,
HTTP integration, or semantic scale gates above.
