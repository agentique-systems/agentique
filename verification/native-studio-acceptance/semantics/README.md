# Semantic candidate acceptance work

The unchanged main command oracle was built and executed first from source
`98c881f0bbac2c55e2777664633a50cb67ab7bf0`, with a clean tracked diff at launch.
The accepted installed KerML/Systems runtime caches were used. The release build
passed (8m19s), and the real CreatePartUsage candidate and all 16 malformed reuse
rejections reached the independent cold checkpoint oracle. That cold reconstruction
failed on a 631,440-byte allocation before final equivalence or phase JSON emission.
The process exit was `0xc0000409` / `3221226505`; this is a failed gate, not a pass.

Exact command/output/exit records: `baseline-command-oracle.json` and
`baseline-command-oracle.log`. The wrapper measures its direct cargo process; its
reported 11 MB RSS is **not** semantic process memory. The separate
`baseline-memory.json` measures the actual test executable and records its SHA-256.
The OS peak working set was **6,226,927,616 bytes**. Its monitor attached after launch;
the OS peak counter includes the earlier lifetime. `baseline-resource-context.json`
records low disk and Windows virtual-memory headroom. No competing semantic process
or compiler was observed during the runtime, while ordinary desktop apps remained
active. The full compile+test command took 1,079.219 s. No new latency or equivalence
result is claimed from this failed run.

The retained previous mission profile remains historical evidence: approximately
64 s final closure, 35 s effective audit, 13 s source preparation, 118 s complete
compile. It motivates the proposed changes but does not substitute a passing rerun.

## Proposed change and required qualification

Identity-proven service checkpoint restoration now passes the exact predecessor
through existing immutable declared-fragment reconstruction. Every blob and syntax
arena is still checked. Source-only restore has no predecessor and remains the oracle.

A successful strict effective-audit outcome can be retained only when both contexts
have authenticated full producer closure; the static KerML/SysML contracts match;
all captured positive and negative/provider read subjects have exact unchanged
record/incoming/proof/search signatures; and every used closure requirement is
proved in the new certificate. Ordinary global reads still invalidate globally.
Direct audit dispatch and mayTimeVary property checks include the subject itself.
All typed queries, supporting queries, names and canonical observations flow through
the existing strict dispatcher. Failed outcomes are reevaluated.

This transports audit counts, not QueryResults or old revision-bound explanations.
The real command oracle now compares complete strict-audit counts/findings and emits
preparation measurements before starting its cold oracle, preserving diagnosis if a
later stage fails. Subject reuse and actual query/family checks reused are separate
counters; zero-query subjects cannot imply useful semantic work elimination.
Producer closure remains required. Checkpoint capture and rebind now have separate
inclusive phase timings.

Required next gates (not yet executed due host disk/memory exhaustion):

- New closed-audit negative-search/global-read/context-binding/closure unit tests.
- Existing strict audit observer parity and workspace checkpoint tests.
- Real command/cold reconstruction exact equivalence and both validations.
- Main workspace formatting, Clippy, tests and publication-input freshness handling.

The next performance concern is full dependency signature traversal and producer
checkpoint rebinding onto an empty derived overlay. Those costs must be measured;
no incremental speedup is claimed until the real wall-time oracle passes.

## Independent oracle memory correction

After the allocation failure, the command oracle was changed to launch command and
cold reconstruction in separate child processes. They compare exact per-record,
occurrence, submitted-slot, derivation/search/contribution/failure, source/reference,
strict-audit and all applicable local effective-query SHA-256 observations. Query
Debug observations retain every value, completeness field and full proof/read
payload, including private additional-completeness state. Only the fresh kernel
revision label is normalized. Both children must validate; equal counts are never
substituted for equality. No parent process retains a semantic model.

Set `AGENTIQUE_CREATE_PART_ORACLE_OUTPUT` to a retained verification/generated
directory when running the test to keep checkpoint, sources, per-observation hashes,
per-child timings and final comparison JSON. Candidate and cold compilation timers
exclude these comparison exports and are printed before export starts. The parent
returns failure if either child fails or any exact observation differs.

The audit signature implementation factors physically identical immutable
publication records/occurrences/proof/search/navigation/contribution rows out of
repeated hashing. Exact publication/context identity remains mandatory. Local
carriers and output proof support still update shared target signatures. This is
not a source-membership assumption: the fast path requires actual pointer identity
with the authenticated dependency. A dedicated adversarial test compares the full
and factored invalidation frontier when a new local typing targets a standard
element. Existing producer checkpoint signatures retain their original behavior.
The audit setup, producer checkpoint and producer rebind costs are separately
reported; testing/measurement of the draft remains pending integration.
