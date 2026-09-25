# Runtime recovery evidence

This stream separates accepted semantic authority from cache recovery and release
transport. Original receipts, profiles and binding manifests are unchanged.

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
`kerml-recovered-content.json` retains the content inventory. A complete
authenticated runtime pair is still pending.

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

The inactive compiled transport mechanism in commit `5007ea5` preserves the
original semantic receipt and bindings. It requires an original-receipt SHA-256
anchor, identical identity and entry bounds, and unchanged facade/closure bytes;
only a reviewed kernel transport digest can differ. No alternate hash is
registered at this checkpoint. Five Rust trust tests pass in both the independent
integration debug gate and release build, with a final unchanged-source release
repeat retained in `transport-receipt-rust-tests-final.json`. Workspace formatting
and all-target Clippy for `agq-kerml-semantics` pass. Independent source review is
retained by the integration branch; actual transport registration and restoration
still await the Systems result.

The manual `runtime-asset` workflow authenticates an explicit draft asset against
its independently recorded transport SHA-256 and both language facades. It has
not been dispatched and does not publish releases. A downloadable runtime is not
claimed until a real pair passes and is packaged.
