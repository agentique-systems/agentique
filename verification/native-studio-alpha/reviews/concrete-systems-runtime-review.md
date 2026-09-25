# Independent review: concrete Systems runtime transport

The exact transport registered by `e44015b3982331c9edca5633ea7f7b6ef85dd6e8`
is eligible for ordinary facade authentication and runtime packaging attempts.
No blocking defect was found in this concrete registration. This review does
**not** accept the runtime: current catalogue-backed facade restoration remains
a separate, mandatory gate.

## Scope and evidence

The reviewer did not author the producer, transport mechanism, preparer, or
registration. The earlier [mechanism review](runtime-transport-independent-review.md)
examines their trust boundary and the subsequently fixed concurrent-file-change
defect. This review inspected the actual generated cache and the finite pin,
rather than accepting their filenames or copying preparer results.

The reviewer-authored [audit script](audit-concrete-systems-transport.py) streamed
all ZIP entries and independently checked the outer transport, accepted contract,
bindings, retained producer binary, closure journal, and command logs. The
[first result](concrete-systems-transport-review.json) passed in 8.234 seconds.
The [registration result](concrete-systems-registration-review.json) additionally
checks that the checked-in pin and all retained evidence copies exactly match
the reviewed generated files. Both commands exited 0. The latter receipt records
the source commit and script/source/artifact hashes used for that run.

The reviewed ZIP is 134,322,830 bytes with SHA-256
`02ba47ad99ceaddb853b198a556cbe40351a00361f077099a993f3a226bcc323`.
Its exact entry population is:

| Entry | Uncompressed bytes | SHA-256 | Accepted transport comparison |
| --- | ---: | --- | --- |
| `closure.json` | 6,759,870 | `0e7527e9f1f49dc8a338dcaaba9b6af3ea92e03ad8818464a957f7a77fef3dc9` | Original digest and length |
| `facade.json` | 19,746,846 | `7a7e489805127b4fd057e95db58d19d53fb15ecaf5a0c2b0af871830cf6967e5` | Original digest and length |
| `kernel.jsonl` | 748,560,767 | `ec9d1c78d864d4ad2f671f93371c1988bcb8b952651b82afb9f3624b0756b828` | Original length; finite alternate digest |

The checked-in transport receipt SHA-256 is
`12cc2aeff7aa7e4a1f3de16faf13d63aa158dd88248e9b3179ad4629e1ffeda0`.
It anchors the unchanged original semantic receipt SHA-256
`becc3cf991e69115dae905957d74292774164e3f5707eecefb06d8eb60e0269a`.
Every field of the generated semantic contract, excluding transport entries,
equals the accepted contract with JSON value types preserved. All 69 generated
bindings are byte-identical to the original binding file, SHA-256
`dbd794bba9ebdfdea5af433945f6bb5d0475f0f05ad1d229cd004f5b57045789`.

## Source and authority boundary

Commit `e44015b` activates one literal, compiled transport receipt for the existing
`sysml-systems-operational-v3` catalogue entry. Its only production Rust change
replaces the empty alternate-transport list with that `include_str!`. The added
test retains the original semantic receipt, accepts this exact graph transport
digest, and rejects a one-bit change. It does not modify restoration logic,
accepted publication authority, operational profiles, source identities, or
bindings. Follow-up `6a6f07f` changes release build concurrency and evidence
documentation only; it introduces no additional trust change.

The retained historical producer checkout remains at
`4ac9b8e58695ad837fed6643b0cedfac05d15635` with no tracked modification. The
review independently rehashed its actual executable and the finalizer journal,
and verified the four successful command receipts against their retained logs.
The historical report records complete closure, all mandatory references,
all accepted bindings, and zero audit findings. Its identity fields match the
original accepted contract.

The original Systems graph payload remains unavailable. Its alternate digest
cannot be explained conclusively as a snapshot-label-only change. Neither the
review nor the registered receipt asserts recovery of those original bytes.

## Remaining acceptance gate

The Python audit establishes concrete transport and contract equality; it does
not independently execute the semantic query engine. The historical producer's
`candidate_cache_restored` result is not proof that the current ordinary facade
accepts the finite compiled transport pin. Current catalogue-backed restoration
must still decode and authenticate the real KerML and Systems caches, including
the canonical semantic graph, closure context, publication identity, and accepted
bindings. Any failure there rejects runtime acceptance despite this review.

After successful ordinary restoration, retain pack/verify/offline-install
receipts tied to the compiled source and actual runtime bundle digest. Native
Agentique first light and durable candidate validation/commit remain product
acceptance gates beyond transport review.
