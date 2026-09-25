# Independent ordinary runtime authentication review

The runtime artifact gate has passed. The current ordinary CLI authenticated
both accepted facades, packaged the exact reviewed pair, verified the resulting
bundle in a separate process, and installed it in the normal offline store.
This establishes runtime availability; Native Agentique model acceptance remains
a separate journey and durable restart gate.

The [independent audit](audit-installed-runtime.py) passed with exit 0 in 2.265
seconds. Its [receipt](installed-runtime-boundary-review.json) records hashes of
the actual executable, critical build-source files, command receipts, logs,
bundle, embedded caches, and installed caches. It compares source files with
build commit `23b7910`, confirms that the reviewed finite pin predates that build,
and verifies that all three ordinary CLI command outputs reached both facade
authentication phases and exited successfully. The reviewer did not rerun the
expensive facades; the retained command executions are the semantic evidence.

| Ordinary command | Wall seconds | Exit |
| --- | ---: | ---: |
| Pack | 86.453 | 0 |
| Independent process verify | 84.156 | 0 |
| Offline install | 75.156 | 0 |

Verification measured KerML restoration at 41,610 ms and Systems restoration
at 38,800 ms. Installation measured 36,406 ms and 34,756 ms respectively.
The actual bundle is 610,190,454 bytes, SHA-256
`37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
The installed bundle identity is
`633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`.
Both embedded caches and both installed caches independently match the exact
transport hashes in the [earlier concrete review](concrete-systems-runtime-review.md).

Source inspection confirms that `runtime-publications::restore` loads the pinned
source library set, calls `CanonicalKermlStandardLibraries::restore_cache`, then
`CanonicalSysmlSystemsLibrary::restore_cache`, and propagates either failure.
`pack_bundle` invokes this path before publishing its output, and verification
and installation also use ordinary restoration. No manifest field bypasses it.

The review also approves the precise freshness update in `1edf050`: the
non-authoritative ledger changes only the reviewed trust-source hash and adds
the exact compiled transport JSON hash. Every authority field and the previous
Cargo lock pin remain unchanged. The freshness checker now inventories transport
JSON files; addition, alteration and removal have negative regression coverage.
The actual regression and standards-check receipts both pass, with retained logs
independently rehashed by this audit. This records transport implementation
freshness and does not reissue semantic publication acceptance.

Original Systems graph bytes have not been recovered, and no UUID-only transport
difference is claimed. Native real-model first light, screenshots, candidate
validation/commit/restart, and remote distribution remain outside this result.
