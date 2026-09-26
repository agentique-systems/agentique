# Audit batch experiment, 2026-09-26

All three immutable-revision cold trials passed exact comparison. Increasing
batch size 32 to 256 saved only 1.654 seconds of effective audit and increased
memory. Keep the shipping batch size 32.

See [the measured analysis](../semantics/audit-batch-result.md) for timings,
memory, limits and provenance. The complete raw result is
`measurements/result.json`; each batch directory contains exact metrics,
commands, exit codes, logs, memory samples and authoritative inputs.

`independent-verification.json` records a second local streaming verification
of the artifact, executable, build outputs, inputs and all observation bytes.
Each of the three 105,985,804-byte maps is exactly identical, including the
previous passing cold oracle. They are retained once as
`observations/cold-observations.json.gz`; the manifest maps each original path
to those same decompressed bytes. No semantic runtime was run locally for this
retention step. `retain_evidence.py` records the exact verification operation.

CI run: https://github.com/agentique-systems/agentique/actions/runs/36235278112

Experimental source: `167cbeca9f4c6e276c68459e51fb16b62e3b383c`.
This experiment predates the corrected main incremental audit implementation.
