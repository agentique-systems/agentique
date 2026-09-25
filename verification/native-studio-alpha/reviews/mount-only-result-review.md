# Independent review of the mount-only reconstruction result

The exact accepted-runtime Create Part oracle passed: one test, all 16 malformed
requests refused, same authenticated predecessor mount observed, independently
different cold mount observed, exact reconstruction equivalence and both real
validations passed. The executable was SHA-256
`28b7038a7644a1f0ad242d0f769739142d72f897c151b9a9e24bf650c259f8a4`;
its launch source commit was `522180aa4ac65dbde79131edd5df8430a17c73b9`.
Evidence: [process receipt](../performance/create-part-shared-mount.json) and
[complete log](../performance/create-part-shared-mount.log).

| Operation | Measured wall time |
| --- | ---: |
| Accepted pair restoration | 71.941 s |
| Shared-mount command prepare | 133.862 s |
| Command compilation, included in prepare | 122.969 s |
| Independent cold checkpoint restoration | 126.323 s |
| Cold compilation, included in restoration | 123.666 s |
| Command validation path | 20.128 s |
| Cold working revision validation | 5.310 s |

The process wrapper measured **626.839 s**, with peak working set
**6,397,239,296 bytes**. The test harness reported 625.91 s for its own interval.
Neither is a per-edit allocation measurement. Both reconstructions reparsed six
documents, rebuilt 1,101 local records, evaluated 717 producer subjects and
audited 791 subjects. Reused records and lowering cache hits were zero; semantic
caches were not used. All 76,153 canonical elements, including accepted library
dependencies, remained part of the identity/evidence comparison.

The old executable recorded 159.459 s prepare and 144.408 s compilation. This
run's prepare is 25.597 s lower, while compilation itself is 21.439 s lower.
Most of the observed difference therefore lies inside unchanged full compilation.
Different runs and host conditions prevent attributing the decrease to mount
sharing. The mixed prepare-minus-compile residual decreased from about 15.051 s
to 10.893 s; that interval also includes source proof, identity continuity,
serialization and other service work. It is not an isolated mount measurement.

Within the new run, hot command prepare is 7.539 s longer than cold checkpoint
restoration. The command performs extra source proof and continuity postchecks;
the cold restore is an independent semantic oracle rather than a second complete
operator command. Likewise command validation includes its service candidate
preparation/serialization path, whereas the cold timing invokes working-revision
validation directly. These intervals are useful observations, not like-for-like
algorithm or validation speed ratios. The larger whole-process peak than the old
5,925,900,288-byte result does not establish an isolated regression or improvement.

Source review confirms the cold path reparses exact durable sources through the
unchanged restore implementation. Its comparison covers canonical identity,
provenance/order, checkpoint, closure, references including kind/alias/visibility,
and full query context/evidence. Only the fresh nested KerML revision label is
normalized where the reconstruction contract requires it. Negative cases cover
parent/project/root/publication and source/identity corruption. No validation,
closure or audit is skipped and no local incremental semantic speedup is claimed.

This result qualifies the bounded shared-mount reconstruction path. It does not
qualify held audit `0cbc54a`, the independent view-query reuse optimization, durable
source-only restart, or the complete native operator journey. Their gates remain.
