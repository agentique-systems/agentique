# Accepted-runtime Create Part baseline

The real six-document Agentique model reached Validated, then the ordinary
`CreatePartUsage(alphaObserver)` command produced a Working candidate. Canonical
records, IDs, relationship order, provenance, closure identity, references and
effective-query evidence matched the independent full rebuild. Both candidates
validated; the current revision and durable branch head remained unchanged.

| Measured operation | Wall time |
|---|---:|
| Accepted runtime restoration | 70.090 s |
| Command prepare, including source proof and reconstruction | 159.459 s |
| Command compilation | 144.408 s |
| Full rebuild over the same parsed source identities | 143.453 s |
| Candidate validation | 22.029 s |

The candidate contained 76,153 canonical elements including its accepted
dependencies. Local construction rebuilt 1,101 records, reparsed six documents,
evaluated 717 producer subjects and audited 791 subjects. Command phase counters
reported reference refinement 12.731 s, declared construction 1.803 s, final
closure 73.651 s, final references 7.572 s and effective audit 48.022 s.
Source preparation was 14.549 s outside the compilation timer. These boundaries
are the compiler's own observations; phases are not an allocation profile.

The whole test ran for 699.466 s with peak working set 5,925,900,288 bytes.
That peak includes restoration, seeding, two candidates and validation; it is
not per-edit memory. A native unit-test build overlapped initial runtime loading
and finished before command preparation. The host was not otherwise isolated.

This baseline does **not** demonstrate incremental semantic speedup. The
comparison rebuild already retains parsed inputs and their mounted standard
dependency, whereas the command reconstructs its source checkpoint. The proposed
optimization will compare a shared dependency mount with an independently cold
checkpoint restoration over identical sources, with exact semantic equivalence.
Local lowering, producers, closure and audit remain full reconstruction.

Exact [process record](create-part-before-sharing.json),
[unabridged test output](create-part-before-sharing.log) and
[command receipt](../checks/create-part-before-sharing.json) are retained. The
baseline executable was preserved before any rebuild, SHA-256
`ea007d101d7ed846038415c6ff8f31e41b1c30e8fb20eb2e6eac70dd5e49678d`;
the [preservation receipt](../checks/preserve-baseline-oracle-fixed.json) records
its absolute local path and byte comparison.
