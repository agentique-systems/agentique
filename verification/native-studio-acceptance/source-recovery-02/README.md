# Optimized separate-process source-only restoration

Passed with the Windows CI release helper built at `a6e1f41d` and SHA-256
`5ee44d57f2ce4d103c58f59698383d6ddf88556035573743abc9134a9cca6c99`.
Build provenance is in `../ci-helpers-01/build.json`.

This independently launched process restored the existing two-revision recovery
repository created by `../source-recovery-01`, whose semantic caches had already
been deleted. It used its retained authoritative sources, identities and accepted
publications, then compared the exact durable manifests, checkpoints, semantic
fingerprints, System/Graph/Requirements projections, projected Inspector answers
and durable head with the original expectation file. Both revisions matched.
No new cache deletion or rewriting of the expected observations occurred.

Actual exit code: 0. Total process wall time: 180.324 s. Maximum process working
set: 5,080,182,784 bytes. No concurrent local compiler, test or other semantic
workload ran. This is correctness and recovery evidence; the small timing change
from the earlier 193.609 s run does not establish the requested restoration win.
