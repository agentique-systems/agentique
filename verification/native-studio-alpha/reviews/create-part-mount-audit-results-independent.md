# Independent combined CreatePart reconstruction result review

Reviewed actual `create-part-shared-mount-and-audit` artifacts, 2026-09-26.
The reviewer started no build, test, native process or accepted-runtime consumer
and changed no acceptance pin.

## Result and exact artifacts

The real accepted-runtime oracle passed: one passed, zero failed, zero ignored,
zero filtered out. The test reported **590.23 seconds**; externally measured
process wall time was **591.076900 seconds**. Maximum process peak working set
was **6,500,315,136 bytes** (6.0539 GiB). Sampled process-tree resident peak was
6,493,577,216 bytes with a 250 ms sample interval. These are process-wide
measurements, not per-edit allocation or candidate-only memory.

The command receipt records source
`b10fe62c7dbb62981a4cd3719820e9631ed99381`, no tracked changes at start,
`2026-09-25T22:31:00.900219+00:00`, and exit 0. Independent SHA-256 checks
matched the receipts:

| Artifact | SHA-256 |
| --- | --- |
| Actual test executable | `61555bb696d1adab8378404b597c0b557c24f0c37f1af0fda2dba87afbed52a5` |
| Command output | `0052baa6dd4e14f4bcef35d273fc30617ca8e2a127789a49038c214ca5b91d67` |
| Performance log | `fe8f269ac439e6f36da055c70696cfc72df92ebaf36d0591a6bf517eb2ad35ae` |
| Build output | `9fe90c67e25df52ae8543440df60016e93620f3bb202caaad589a1dff82af876` |

No crate, model or standards source changed between build commit `7270d6e5`
and invocation commit `b10fe62c`. The six model documents are also unchanged
from the earlier mount-only run at `522180aa`. The read-only artifact, source
and metric verification completed with exit 0.

## What the semantic oracle establishes

The test restores the exact accepted caches through ordinary language facades,
creates a repository and requires the real six-document self-model to reach
Validated. The ordinary agent command creates `alphaObserver` under the actual
ModelingPlatform definition. The owner and containing architecture retain their
canonical IDs; the added part has the exact canonical owner and source edit.
The predecessor fingerprint and durable branch head remain unchanged while the
candidate exists.

The command reconstruction must share the predecessor's authenticated dependency
mount. The comparison restores the resulting checkpoint with identical source
and identity inputs using an independently authenticated mount; a pointer-sharing
assertion rejects accidentally reusing the command mount for the cold oracle.
The pair then requires:

- Exact source/arena/retirement checkpoint and semantic fingerprint.
- Exact canonical records, IDs, provenance, association identity and ordering.
- Exact semantic closure digest.
- Equal reference population, reference metadata, resolutions and public query
  evidence/dependencies/completeness, normalizing only the fresh kernel revision.
- Equal diagnostics under that same revision-label normalization.
- Identical added-part lookup and effective SysML answer, including supporting
  queries, names, observations, pending implications and diagnostics.
- Successful ordinary candidate validation and cold reconstructed validation.

All **16** malformed reuse requests are explicitly logged as rejected: checkpoint
version, absent/foreign parent, reused revision, source version, foreign project,
foreign root, foreign KerML/Systems identities, changed/missing/substituted source,
duplicate document identity/path, duplicate syntax identity, and incomplete arena.

This proves candidate reconstruction and validation equivalence for the exercised
command. It proves preservation of the durable predecessor, but does **not**
commit this candidate or restart a new service/process. The native committed-part
journey and separate restart remain the durable created-part acceptance gate.

## Actual phase measurements

| Measurement | Command with shared mount | Independent cold restore |
| --- | ---: | ---: |
| Whole preparation / restore | 128,619 ms | 121,136 ms |
| Authored compile | 117,858.911 ms | 118,516.066 ms |
| Source preparation, inclusive | 12,719.227 ms | 12,653.394 ms |
| Reference refinement, inside source preparation | 11,228.987 ms | 11,120.396 ms |
| Final closure, inclusive | 64,299.851 ms | 64,390.255 ms |
| Certificate construction, inside final closure | 21,340.634 ms | 21,411.245 ms |
| Final reference resolution | 5,498.566 ms | 5,515.349 ms |
| Effective audit | 34,835.525 ms | 35,439.683 ms |
| Subsequent validation call | 20,307 ms | 5,511 ms |

Accepted-runtime restoration took 60,905 ms. Parsing occurs before the authored
compile timer. Preparation-minus-compile residuals are mixed source/checkpoint,
proof, serialization and postcheck overhead: 10,760.089 ms for the command and
2,619.934 ms for cold restoration. They do not isolate mount time. The command
and direct cold validation calls have different enclosing responsibilities;
their wall times are not interchangeable. Nested source/certificate phases
must not be added again to inclusive totals.

Both reconstructions contain 76,153 canonical elements and six authored
documents. They report identical work: six reparsed documents, 18 document
lowerings, 1,101 rebuilt records, zero reused records, zero
lowering-cache hits, 791 audited subjects and 717 producer subjects. Semantic
cache use is false. The final closure counters also agree apart from elapsed
certificate time: seven rounds, 1,403 closed producer pairs, zero incomplete
pairs and 456,918 closed effects. This is full reconstruction with accepted-mount
reuse and avoided duplicate audit-answer delivery, not incremental semantic
work elimination.

## Comparison limits and interference

The prior mount-only observation recorded preparation 133,862 ms, compile
122,969.099 ms, cold restore 126,323 ms and cold compile 123,666.414 ms. The
combined observation is lower by about 3.92%, 4.16%, 4.11% and 4.16%, respectively.
Audit observations changed from 39,666.877 / 39,669.667 ms to 34,835.525 /
35,439.683 ms. The work counters are identical across both pairs. These are
single-run observations; they do not establish a causal aggregate speedup.

The parent reports no other accepted-runtime consumer or build during the
combined measurement. It was not a strictly idle machine: an agent downloaded
593,320 bytes via CI HTTP Range at 22:33:05 UTC, and the parent created another
source worktree, checking out 5,145 files in 4.65 seconds during approximately
the final minute before completion at 22:40:52 UTC. This limited activity does
not weaken exact semantic assertions, but it constrains timing attribution.
No repeated A/B distribution, per-edit memory reduction, UI latency, p95 or
60 Hz claim follows from this oracle.

The result is a qualified semantic success with a modest observed wall-time
change. Candidate preparation remains approximately two minutes. Real native
responsiveness during that work is a separate input/query qualification; full
candidate commit and durable restart remain outstanding for product acceptance.
