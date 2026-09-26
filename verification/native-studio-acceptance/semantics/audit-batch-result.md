# Effective-audit batch size: measured result

Keep the shipping batch size at 32. Increasing the batch to 256 reduced this
real-model cold audit by 1.654 seconds (3.45%) and checkpoint restoration by
1.253 seconds (0.79%), while increasing process memory. Batching does not explain
or solve the remaining multi-minute semantic latency. This is a completed
negative optimization experiment, not an incremental-edit speedup.

## Execution and scope

[Windows CI run 36235278112](https://github.com/agentique-systems/agentique/actions/runs/36235278112)
completed successfully on 2026-09-26. Source commit
`167cbeca9f4c6e276c68459e51fb16b62e3b383c` is isolated on
`platform/native-studio-alpha-semantic-experiment`. It predates the corrected
incremental audit fingerprints and factored producer checkpoint in main commit
`42e7e187de823a3b855d530dcee84dbe8c06b59f`.

The experiment used one release executable and one Windows runner with
17,174,360,064 bytes of physical memory. The same authoritative checkpoint and
source bytes from the prior passing independent oracle were copied unchanged
into each trial. Each fresh child independently authenticated the accepted
runtime, reconstructed the checkpoint from source, ran the complete effective
audit and validation, and exported the complete semantic observation map.
The trials ran sequentially with batch sizes 32, 128 and 256. No semantic child
overlapped another child. Production builds still use 32; the environment
override is available only with the verification feature.

All commands, environments, outputs and exit codes are retained in the artifact.
The executed command for each trial was:

```text
create_part_performance.exe create_part_command_matches_full_self_model_reconstruction --exact --ignored --nocapture --test-threads=1
```

The stage was `AGENTIQUE_CREATE_PART_ORACLE_STAGE=cold`; the batch environment
variable was `AGENTIQUE_AUDIT_BATCH_SIZE=32`, `128`, or `256`. Every command exited
0. Every child reported one passed test and successful independent validation.

## Timing

| Measured seconds | 32 | 128 | 256 |
| --- | ---: | ---: | ---: |
| Effective audit | 47.872384 | 46.818571 | 46.218561 |
| Final semantic closure | 82.867403 | 83.054637 | 83.254318 |
| Full source compilation | 154.660751 | 153.786397 | 153.370808 |
| Checkpoint restoration | 157.933 | 157.085 | 156.680 |
| Accepted runtime restoration | 71.004 | 70.968 | 71.176 |
| Subsequent validation | 3.320 | 3.344 | 3.354 |
| Whole child process | 596.094 | 581.969 | 593.734 |

Compilation contains closure and effective audit. Checkpoint restoration
contains compilation. These rows must not be added together. The whole child
process also includes the subsequent large exact-proof observation export;
it is not the candidate preparation time. Its nonmonotonic result does not
establish a reliable larger-batch throughput benefit. There was one trial of
each size, so this evidence does not estimate run-to-run variance.

All three audits evaluated 791 subjects and reused zero audit subjects/checks.
All closures evaluated 717 producer subjects, ran seven fixed-point rounds and
materialized five overlays. Closure work counters were identical apart from
elapsed time. The batch count changed from 25 to 7 to 4. This experiment tests
memo retention within one immutable revision; it does not test cross-revision
incremental reuse or the native candidate path.

## Memory and equivalence

| Whole-process memory bytes | 32 | 128 | 256 |
| --- | ---: | ---: | ---: |
| Peak resident working set | 5,955,940,352 | 5,986,619,392 | 6,185,918,464 |
| Peak private allocation | 6,666,133,504 | 7,042,224,128 | 7,118,049,280 |
| Minimum available host memory | 8,897,179,648 | 9,002,418,176 | 8,665,903,104 |

The observer sampled every 250 ms and included the OS peak working-set counter.
Memory covers the entire child, including proof export, and therefore is not
an isolated effective-audit peak. No memory reserve, timeout or termination
guard fired. Compared with 32, batch 256 increased peak resident memory by
229,978,112 bytes (219.3 MiB) and private memory by 451,915,776 bytes (431.0 MiB).

All 697,419 observations, including 1,633 query observations, compared exactly:
no missing, extra or different entries. Batch 32 also matched the pinned prior
passing cold oracle. The complete observations cover canonical records,
relationship occurrences, identities, derived facts and proof data, effective
query values/evidence/completeness, closure certificate digest and validation
diagnostics. Only fresh kernel revision labels are normalized by the existing
observer. All three files have the same reported SHA-256:
`0beea518acd35a012856c938f0e34ee90ba07b098b0e274d4d6d4d725e6f508a`.

## Provenance and retained evidence

The artifact is
`audit-batch-experiment-windows-167cbeca9f4c6e276c68459e51fb16b62e3b383c-1`
(ID `10905095102`). Its ZIP SHA-256 was independently verified against GitHub's
artifact digest before bounded extraction:
`1c37ac94f6338477bb4cebede7724ac2f73c5a40e21a35bb07525c39fc4da775`.

| Input or build product | SHA-256 |
| --- | --- |
| Release oracle executable | `c4e680aea3f952b62118656c897d592f9db097aca056333c88056a0a4b734578` |
| Build receipt | `ac5ab579bde0060c6a33fed3e91b2205c1a2528f5a2ecdb3864d80688c5126b8` |
| Authoritative checkpoint | `740da901d19d316112b5e87d041f2605e990b1669a589aa000a9a5148c30c3a0` |
| Sources | `1e65f97ba88ea825250adc834de28fa192a58b761904b275f0268492d93315b5` |
| KerML runtime cache | `aadf9ff589eea35bcedc5d1e3d0521952058b55f3325614ecb0ab12ae36300c4` |
| Systems runtime cache | `02ba47ad99ceaddb853b198a556cbe40351a00361f077099a993f3a226bcc323` |

Raw downloaded evidence is under
`verification/generated/native-studio-acceptance/audit-batch-01`:
`measurements/result.json`, each batch's `process.json`, `process.log`,
`memory.jsonl`, `cold-metrics.json`, and `cold-observations.json`, plus the
executable, build receipt and authenticated input provenance. Local download
and extraction overlapped a separate local native journey; the measured trials
had already completed on the remote runner, so the overlap cannot affect their
reported times. After that native journey ended, independent local streaming
verification returned exit 0: all three complete observation files were
byte-identical to each other and to the previously retained passing cold map.
Build command outputs, executable, input receipts, checkpoint/source bytes and
process log hashes also matched their receipts.

Reviewable evidence is retained under
`verification/native-studio-acceptance/audit-batch-01`, including
`independent-verification.json`, all raw metrics, logs, memory samples, source
inputs and build provenance. `observations/cold-observations.json.gz` stores the
one byte-identical map shared by the three trials. Its decompressed bytes were
verified after compression. The authenticated generated ZIP and executable are
identified in the retention manifest but are not committed as binaries.
