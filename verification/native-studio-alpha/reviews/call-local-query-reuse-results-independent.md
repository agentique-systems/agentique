# Independent review of the completed query-reuse oracle

The accepted-runtime oracle passed, and the measured focused projection and
Inspector calls each took approximately 47.5% less time. This qualifies the
call-local query reuse on the tested immutable Agentique revision. It does not
establish UI latency, p95, candidate reconstruction speed, or product acceptance.

Evidence: [execution receipt](../performance/view-query-reuse.json),
[complete log](../performance/view-query-reuse.log),
[command receipt](../checks/view-query-reuse.json),
[ordinary regression results](../checks/view-query-reuse-regressions.txt), and
[prior independent source review](call-local-query-reuse-independent.md).
This reviewer read the actual results and implementation, recalculated the
statistics, and verified the recorded executable and command-output hashes.
No build, runtime consumer, or semantic query was launched by this review.

## Measured comparison

Three alternating baseline/current pairs followed correctness warm-up. Both
implementations used the same restored revision, fresh per-call evaluators and
disabled application profiling. DTO comparison and serialization were outside
the timed calls. The baseline implementation was preserved from
`d404a8759891acc5d621dcad655fa4a981e17d63`; the optimized code was reviewed as
`20840905a0a1554b1106357b2cb6230f77ac279e`, integrated as `bcf354a`.

| Full call | Baseline median | Current median | Time reduction | Ratio |
|---|---:|---:|---:|---:|
| Focused ModelingPlatform projection | 5,536.424 ms | 2,901.396 ms | 47.59% | 1.908x |
| ModelingPlatform Inspector | 5,559.806 ms | 2,920.386 ms | 47.47% | 1.904x |

Projection ranges were 5,528.596–5,540.641 ms before and
2,895.725–2,910.255 ms after. Inspector ranges were 5,554.951–5,567.478 ms before
and 2,901.478–2,930.590 ms after. These are three observations per implementation,
not a percentile distribution or a statistically qualified population estimate.

The separate three-round experiment rotated fresh, forked and shared query
evaluators after identical connector-query priming:

| Arm | Median context construction/fork | Median focused-query work |
|---|---:|---:|
| Fresh context and evaluator | 2,648.192 ms | 205.080 ms |
| Forked immutable context, empty evaluator caches | 0.0012 ms | 198.858 ms |
| Existing primed evaluator | No constructor | 200.299 ms |

`KerMlQueries::fork` creates a new evaluator with empty traversal/evidence
caches over the exact same immutable context. The shared arm retains the
primed evaluator. Thus these observations support avoiding repeated context
construction as the principal explanation for the approximately 2.64-second
call reduction. They do not establish a material benefit from retaining the
connector-query caches: the three focused-query medians remain near 200 ms,
and shared was not faster than fork in these observations. Priming, evaluator
destruction and output comparison are excluded from the arm timings; use the
full paired calls above for overall call latency. Do not add the arm medians
and describe the result as another measured full call.

## Exact-result qualification

The successful final assertion was reached after all 17 projection cases and
eight Inspector cases. The oracle compares entire DTO values and serialized
bytes, or exact error variants/payloads and rendered messages, without identity
normalization. Cases include focused/overview Architecture, standards and hidden
elements, connection filtering, both graph scopes at depths 0/1/2, Requirements,
an unsupported definition version, and a missing element. Seven actual
Inspector subjects plus the missing-element case cover the system, Platform,
Repository, inherited and owned ports, connector and requirement.

Raw-query checks compare exact contexts, values, completeness, diagnostics,
positive/search/canonical dependencies, explanations and both origin sets, plus
the complete Debug representation for otherwise private evidence state. They
also require the same model reference. Queries cover effective features,
feature owners/types, subsetting, redefinition, supertypes and connector ends
as applicable. The final semantic fingerprint equals the initial fingerprint.

The ordinary regression executable separately passed 19 tests with the runtime
oracle ignored; the explicit ignored invocation then passed this oracle. Those
19 include factory dispatch/fallback/error preservation and a synthetic
incomplete/invalid canonical-query case. Actual query-factory failure parity on
a live Working candidate remains outside this accepted Validated-revision run.
No new standard interpretation, publication acceptance or closure authority is
inferred from the performance result.

## Provenance and limits

- Restored project `c20c4a5c-ea43-45ea-89d2-db2742e2c20a`, revision
  `8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46`, required `Validated` through ordinary
  accepted cache restoration; recorded load path `AuthenticatedSemanticCache`.
  The [closed backup record](../performance/view-query-baseline-snapshot.json)
  preserves committed WAL content and binds the database to the real-run03
  baseline. The oracle copies that backup rather than mutating the live project.
- Recorded source at invocation: `e5aa5cf08ee6998d4b996fae299c86b9dd785c72`.
  Executable SHA-256, independently matched to the current binary:
  `75ee5c7fe298af999b9f71751bab8e8e0c000501b005e5ff7a7842a93c050d0a`.
  Command-output SHA-256 independently matches its receipt:
  `1d71172a927dde230f41e315f04f621112b7b8dc53c1aaaf3ecb37b7f07a8735`.
  Complete performance-log SHA-256:
  `489143703ace09c43e6e812951a1679af92c806c0c2d4a90d0ac296e601deca3`.
- The gate exited 0 after 495.666 seconds. Runtime restore was 61.175 seconds;
  revision restore was 111.776 seconds. Those are setup observations in this
  process, not measured improvements or fresh-process native startup results.
- Maximum process peak working set was 5,536,743,424 bytes (5.156 GiB); the
  250-ms sampled process-tree resident peak was 5,531,361,280 bytes. This covers
  the whole oracle with both implementations and restoration. It cannot be
  attributed to either implementation or claimed as a memory reduction.
- The build used release mode with LTO disabled. No native scene build, layout,
  GPU, frame presentation, input queue, background-reader interaction, disk-cold
  comparison, edit preparation, or validation was timed here. The remaining
  approximately 2.9-second application calls still require the separately
  implemented background interaction and its actual native qualification.
