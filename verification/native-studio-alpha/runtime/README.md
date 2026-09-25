# Runtime recovery evidence

This stream separates accepted semantic authority from cache recovery and release
transport. Original receipts, profiles and binding manifests are unchanged.

Exact command outputs are retained beside their JSON receipts, including
`runtime-pack.log`, `runtime-verify.log`, `runtime-install.log`, both historical
producer replays and the strict Systems finalizer. `output-retention.json` maps
the 32 original recovery logs (684,650 bytes total) to their commands and hashes. Twenty-six logs
match hashes recorded at command completion; six early records did not include
output hashes, so the inventory explicitly records that limitation. The aborted
optional preflight's exit 15 and original output remain visible. Cache archives
and generated bulk are excluded from Git.

The final storage search found no accepted cache candidate. GitHub returned no
releases and zero Actions caches. The newest of 82 workflow artifacts was actually
downloaded: run `36150517543`, 146,311,299 bytes. It contained 3,046 files and only
one ZIP, the retained historical browser trace. A bounded HTTP-range inventory
then inspected the ZIP directories of all 82 retained Actions artifacts: zero
inspection failures and 15,439,400 bytes transferred. Seven broad name matches
were copies of the 23 KB structural-query summary, with no accepted cache payload.
Project-related local ZIP archives contained no publication caches.
`actions-archive-inventory.json`, `search-*.json`, `latest-ci-download.json`,
`latest-ci-inventory.json` and `local-archive-inventory.json` retain the observations.

The exact historical KerML producer at `a1a7b847` compiled successfully in 551.812
seconds. Its binary hashes are retained. The separate Systems producer/finalizer
uses `4ac9b8e`; its language source is identical to original closure `257d73cb`.
Recovery never passes `--write-bindings` or replaces accepted receipts.

At this evidence checkpoint, KerML rematerialization passed its original semantic
contract and exact uncompressed archive entry contract. The full replay exited 0
in 1,819.328 seconds, with sampled peak RSS 5,033,037,824 bytes. It checked all
61,718 capability subjects and 4,000 mandatory references with zero findings,
matched the existing semantic digest and all 31 bindings, then wrote its cache.
`kerml-rematerialization-summary.json` and the actual command record retain this
result. `kerml-original-entry-reproduction.json` records exact equality of the
78,187,642-byte facade and 2,176,394,780-byte graph with the original receipt after
restoring only the historical snapshot revision label. The dependent Systems
producer authenticated that cache through the ordinary KerML facade in 40.882
seconds before replaying its accepted closure. The recovered archive is
475,865,571 bytes with SHA-256
`aadf9ff589eea35bcedc5d1e3d0521952058b55f3325614ecb0ab12ae36300c4`;
`kerml-recovered-content.json` retains the content inventory.

Systems closure completed in 4,026.203 seconds, with 27,177/27,177 closed
producer pairs and 452,172/452,172 closed requirements. The separate strict
finalizer completed in 2,291.282 seconds with sampled peak RSS 5,813,383,168
bytes. Every full capability/effective audit passed with zero findings;
1,327/1,327 mandatory references and all 69 accepted bindings match.
`systems-finalized-report.json` retains the complete emitted report.
`systems-existing-contract-result.json` compares the whole original receipt
apart from transport entries, with exact equality of all semantic fields and
bindings. `systems-independent-artifact-result.json` separately checks the
actual output artifacts and their provenance.

The regenerated Systems facade and closure entries exactly match the original
bytes. Its graph has the same 748,560,767-byte length but a different transport
SHA-256. Original graph bytes are unavailable, so the precise byte difference
cannot be proved to be only a random snapshot label. The new finite transport
receipt pins `ec9d1c78d864d4ad2f671f93371c1988bcb8b952651b82afb9f3624b0756b828`
under the original semantic receipt SHA-256; it does not replace accepted
publication authority. `systems-rematerialized-content.json` retains the
actual snapshot label and every original/new entry comparison.

The current application installer then passed all three ordinary runtime gates:
`pack` in 86.453 seconds, independent `verify` in 84.156 seconds and normal-store
`install` in 75.156 seconds. Every command restored both language facades without
producer replay. Verification separately measured 41,610 ms for KerML,
38,800 ms for Systems and 1,356 ms for transport checks. Installation measured
36,406 ms, 34,756 ms and 1,146 ms respectively. Peak sampled process RSS was
4,839,034,880 / 4,975,308,800 / 5,021,249,536 bytes. These are cache authentication
timings, not Native Studio candidate or real-model reconstruction measurements.

`runtime-manifest.json` preserves the exact package manifest;
`runtime-verification.json` retains the actual commands, binary identity, results
and normal store location. The 610,190,454-byte package has SHA-256
`37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
It is installed at
`C:/Users/phili/.agentique/publications/633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`.
Native real-model acceptance is tracked separately by the integration stream.

The original scoped development preflight was deliberately stopped after
1,046.672 seconds, exit 15, after checking 20,157 subjects with zero findings and
requesting further provider scope expansion. The retained recovery wrapper calls
the unchanged global producer with every original full closure/reference/capability
gate, then requires exact existing semantic digest and binding-manifest equality.
It avoids repeating development scope discovery and post-publication benchmarks.
`optional-preflight-stop.json` records this choice; its stop is not semantic success.
The historical language source diff is empty; the added wrapper source hash is in
`rematerialization-producer.json`.

A process start is not authentication. Completion records appear only
after commands exit, with actual exit codes and wall time; long semantic commands
also sample peak process memory. `rematerialize_kerml.py` and
`rematerialize_systems.py` encode the exact dependent sequence. The Systems stream
waits for strict original KerML payload reproduction before restoring its facade.

The new transport recovery utility allows only the historical random snapshot
revision label to be restored, then demands the original accepted payload hashes
and lengths exactly. Seven rejection/equivalence tests passed; see
`transport-recovery-tests.json` and its unmodified log. They establish the tool's
failure behavior. Four additional Systems contract tests reject changed or extra
identity fields, changed bindings and invalid transport receipt shapes; the
combined suite passed 11 tests in `transport-contract-recovery-tests.json`.
Two further JSON type-identity cases bring the suite to 13 passing tests in
`recovery-json-identity-tests.json`. The real generated KerML receipt also passes
the stricter equality that distinguishes Boolean, integer and floating values.

The compiled transport mechanism introduced in commit `5007ea5` preserves the
original semantic receipt and bindings. It requires an original-receipt SHA-256
anchor, identical identity and entry bounds, and unchanged facade/closure bytes;
only a reviewed kernel transport digest can differ. Commit `e44015b` registers
the concrete result above and adds a real-catalogue rejection test. Before that
registration, five Rust trust tests passed in both the independent
integration debug gate and release build, with a final unchanged-source release
repeat retained in `transport-receipt-rust-tests-final.json`. Workspace formatting
and all-target Clippy for `agq-kerml-semantics` pass. Independent source review is
retained by the integration branch; actual normal restoration is a separate gate
from source review or generated receipts.

The integration lead created the unpublished runtime draft with four assets,
targeting `c00de91f348bc59934c721fd75b17cbd09217163`. The runtime stream then
downloaded all four assets into a fresh ignored directory in 54.125 seconds and
independently checked every byte hash/length in 2.125 seconds. All match both
the authenticated local assets and GitHub-reported digests. The release body
matches the reviewed notes; the embedded manifest equals the separate asset.
`remote-distribution-result.json` and exact `remote-draft-*.log` outputs retain
these results. No publication or asset replacement occurred in this stream.

The manual `runtime-asset` workflow has not been dispatched. A fresh normal
facade run on the downloaded copy is deliberately deferred while the native
real-model journey holds the runtime. Byte equality with the already
authenticated bundle is established; that separate rerun is not yet claimed.
