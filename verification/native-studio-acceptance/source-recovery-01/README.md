# Separate-process source-only recovery

Passed on Windows with the real installed KerML Operational v9 / Systems v3
publications. This isolated project contains two committed imports (.sysml then
.kerml), not the Agentique self-model. `run_source_recovery.py` created the project,
ran ordinary candidate validation/commit, removed only the two semantic cache
blobs, verified that authoritative tables and all five mandatory blobs remained
byte-identical, then launched a separate restore process.

Both restored revisions took the DurableSource path and exactly matched the
saved manifest, checkpoint, semantic fingerprint, System/Graph/Requirements
projections and all projected Inspector observations. Durable head was unchanged.
The original SQLite file is retained under the corresponding generated directory;
the portable JSON observations, command outputs and exit codes are kept here.

The executable's exact SHA-256 is recorded in recovery.json. It was produced by
acceptance-profile-build-02 (source at build start bd72aee4). The launch record's
source commit is newer because other UI fixes were integrated during compilation;
none of the candidate/audit/cache optimization drafts was included in this build.
Creation overlapped release linking and is correctness evidence, not a clean
performance benchmark. The independent restore ran without that compiler and
used about 5 GB peak working set, including the large accepted runtime.

This gate must be rerun with the optimized executable before final acceptance.
