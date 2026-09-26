# Isolated effective-audit batching experiment

`audit-batch-experiment.patch` is unapplied. It changes only the lifetime of
temporary evaluators in a verification build and the independent oracle's
observation batches. Production files in the active checkout remain untouched.
Source identity, file hashes and static checks are recorded in
`audit-batch-experiment-patch.json`.

The default remains 32 subjects. With the `agq-kerml-text/verification` feature,
`AGENTIQUE_AUDIT_BATCH_SIZE` accepts exactly `32`, `128` or `256`. Unsupported
values fail before the audit; they cannot silently become an unbounded batch.
Non-verification builds ignore the variable and keep 32. The existing modeling
agent `verification` feature enables the required text feature transitively.

Each batch still forks the same authenticated immutable context. All subjects,
typed queries, mayTimeVary checks, completeness checks, evidence collection,
invalidation and successful-audit retention remain unchanged. There is no new
shared cache across revisions. For 791 subjects, the three settings create
25, 7 or 4 evaluator batches respectively. Fewer discarded memo tables might
avoid repeated standard/inheritance traversals; longer-lived proof data might
increase peak memory. Neither outcome has been measured for this patch.

When the variable is set, `SOURCE_AUDIT_BATCH_EXPERIMENT` lines record the chosen
size, subject/evaluation/reuse counts, number of batches and inclusive audit
elapsed time. This is diagnostic metadata, not acceptance evidence.

## Measurement sequence

Wait for the active native build and primary golden-edit oracle to finish before
applying or compiling either diagnostic patch. Preserve that primary result.
Then apply this patch independently, or alongside `closure-profile.patch`, and
build the existing `create_part_performance` verification test executable. Keep
the same compiled executable and machine for all three batch measurements.

Use the golden oracle's retained `checkpoint.json` and `sources.json`. Copy those
two files byte-for-byte into three new output directories. Do not regenerate
them per batch: fresh project/element identities would invalidate comparison.
Run one batch setting at a time in a separate process with these environment
variables:

```text
AGENTIQUE_SOURCE_ROOT=<actual checkout>
AGENTIQUE_KERML_CACHE=<authenticated installed kerml.cache>
AGENTIQUE_SYSTEMS_CACHE=<authenticated installed systems.cache>
AGENTIQUE_CREATE_PART_ORACLE_STAGE=cold
AGENTIQUE_CREATE_PART_ORACLE_DIR=<that batch's output directory>
AGENTIQUE_AUDIT_BATCH_SIZE=32
```

Invoke the compiled test binary with:

```text
create_part_command_matches_full_self_model_reconstruction --exact --ignored --nocapture --test-threads=1
```

Use the existing `run_record.py` process-tree sampler around each invocation,
with a unique `--name`, to retain the actual command, output, exit code, whole
process wall time and process memory. Record the executable digest and all
environment values separately. Repeat for 128 and, if the 128 run completes
within available memory, 256. Preserve failures and do not interpret an OOM as
an equivalence result. Do not overlap semantic or compiler workloads.

Each child independently restores the same source checkpoint, validates it,
exports full semantic observations and writes `cold-metrics.json`. The patch
also selects the experimental batch size in `write_observations`, so its 29
applicable effective query families actually run with the tested evaluator
lifetime. Their complete values, evidence, supporting queries/names, canonical
observations, completeness, diagnostics and pending implications are hashed;
only fresh kernel revision labels are normalized. Compare every key and hash in
`cold-observations.json` for 128/256 against the 32 baseline. Also require
`cold-metrics.json["validated"] == true` in every run. Matching graph counts or
audit checked-family counts is insufficient.

The cold child separates `full_phases.effective_audit_micros` and reconstruction
from the later observation export. Report audit time and total reconstruction
separately; proof export is not part of candidate preparation. Whole-process peak
memory includes the independent observation pass and therefore gives a
conservative bound on this experiment's memory, not an isolated audit-only peak.

If a larger batch wins with exact observations and acceptable memory, repeat the
same 32/128/256 setting with `profile_open` on the same retained real database to
measure its effect on persisted restoration. Candidate incrementality remains
the separately required command/cold oracle, not this cold-audit experiment.

## Current evidence

Targeted `rustfmt` on both isolated files and `git apply --check` returned exit
code 0. No build or runtime experiment has been executed. This artifact claims
no faster audit, lower memory use or established batch-equivalence result.
